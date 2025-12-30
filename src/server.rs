use std::{net::SocketAddr, ops::ControlFlow, time::Duration};

use tokio::{net::TcpListener, sync::mpsc};
use tokio::time::timeout;
use axum::{
    Json, Router, 
    extract::{
        ConnectInfo, WebSocketUpgrade, 
        ws::{Message, Utf8Bytes, WebSocket},
    }, 
    http::{HeaderMap, StatusCode}, 
    response::IntoResponse, 
    routing::{any, get, post}
};

use serde::{Deserialize, Serialize};

//allows to split the websocket stream into separate TX and RX branches
use futures_util::{sink::SinkExt, stream::StreamExt};

pub async fn init_server(http_port: u16) -> Result<(), std::io::Error> {
    // TODO: this allocates heap for no reason.
    // at the same time, there's no `help` to direct convert primitive numbers to &str..
    let http_address = format!("0.0.0.0:{http_port}");

    // build our application with a route
    let app = Router::new()
        .route("/", get(root))
        .route("/users", post(create_user))
        .route("/ws", any(ws_handler));

    println!("server is up ");

    // TODO: input given port, if already occupied, return incremented port.
    let listener = TcpListener::bind(http_address).await.unwrap();
    axum::serve(listener, app.into_make_service_with_connect_info::<SocketAddr>()).await
}

async fn root(
    // headers: HeaderMap,
    // ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> &'static str {
    println!("axum received http");
    "root response: 안녕 世!"
}

async fn create_user(
    // this argument tells axum to parse the request body
    // as JSON into a `CreateUser` type
    Json(payload): Json<CreateUser>,
) -> (StatusCode, Json<User>) {
    // insert your application logic here
    let user = User {
        id: 1337,
        username: payload.username,
    };

    // this will be converted into a JSON response
    // with a status code of `201 Created`
    (StatusCode::CREATED, Json(user))
}

#[derive(Deserialize)]
struct CreateUser {
    username: String,
}

#[derive(Serialize)]
struct User {
    id: u64,
    username: String,
}

// =============== WS ===============

/// The actual switching from HTTP to websocket protocol will occur after this.
/// This is the last point where we can extract TCP/IP metadata and HTTP headers etc.
async fn ws_handler(
    ws: WebSocketUpgrade,
    _headers: HeaderMap,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
) -> impl IntoResponse {
    println!("{addr} attempt ws");

    ws.on_upgrade(move |socket| handle_socket(socket, addr))
}

/// Actual websocket statemachine (one will be spawned per connection)
async fn handle_socket(mut socket: WebSocket, who: SocketAddr) {
    // ========== HANDSHAKE START ========== //

    if socket
        .send(Message::Text(format!("Hello this is server").into()))
        .await
        .is_err() {
        return;
    }

    let Ok(Some(Ok(msg))) = timeout(Duration::from_secs(5), socket.recv()).await else { return };

    if process_message(msg, who).is_break() {
        return;
    }

    let (mut sender, mut receiver) = socket.split();
    let (channel_tx, mut channel_rx) = mpsc::channel::<Message>(32);
    let channel_tx_clone = channel_tx.clone();
    // ========== HANDSHAKE END ========== //

    // Spawn a task that will push several messages to the client (does not matter what client does)
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = channel_rx.recv().await {
            let should_terminate = match msg {
                Message::Close(_) => true,
                _ => false
            };

            if sender
                .send(msg)
                .await
                .is_err() || should_terminate {
                return;
            }
        }
        /*  Message::Close(Some(CloseFrame {
                code: axum::extract::ws::close_code::NORMAL,
                reason: Utf8Bytes::from_static("Goodbye"),
            }))
        */
    });

    // This second task will receive messages from client and print them on server console
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(msg)) = receiver.next().await {
            // print message and break if instructed to do so
            
            let (do_break, msg): (bool, Option<Message>) = match process_message(msg, who) {
                ControlFlow::Break(reply) => (true, reply),
                ControlFlow::Continue(reply) => (false, reply)
            };

            if let Some(reply) = msg {
                if channel_tx_clone.send(reply).await.is_err() {
                    break;
                }
            }
            if do_break { break; }
        }
    });

    // If any one of the tasks exit, abort the other.
    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(()) => println!("server's sender socket terminated"),
                Err(b) => println!("Error sending messages {b:?}")
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(()) => println!("server's reciever socket terminated"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
        }
    }
    println!("Websocket context {who} destroyed");
}

/// helper to print contents of messages to stdout. Has special treatment for Close.
fn process_message(msg: Message, who: SocketAddr) -> ControlFlow<Option<Message>, Option<Message>> {
    match msg {
        Message::Text(t) => {
            println!(">>> {who} sent str: {t:?}");
            let msg = Message::Text(Utf8Bytes::from_static("we heard yo"));
            return ControlFlow::Continue(Some(msg));
        }
        Message::Binary(d) => { 
            // INFO: file should be within 65535 byte including header.
            println!(">>> {who} sent {} bytes: {d:?}", d.len());
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> {who} sent close with code {} and reason `{}`",
                    cf.code, cf.reason
                );
            } else {
                println!(">>> {who} somehow sent close message without CloseFrame");
            }
            return ControlFlow::Break(None);
        }
        Message::Pong(_) => {}
        Message::Ping(_) => { /* we don't touch this. */ }
    }
    ControlFlow::Continue(None)
}
