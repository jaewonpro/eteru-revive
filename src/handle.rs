use std::{io::Error, ops::ControlFlow, sync::OnceLock};

use redb::{Database, ReadableDatabase, TableDefinition};
use rkyv::{util::AlignedVec};
use tokio::{runtime::Runtime, sync::mpsc, task::JoinHandle};
use anyhow::{Result, bail};
use tokio_tungstenite::tungstenite::{Message, Utf8Bytes};
use futures_util::{SinkExt, stream::StreamExt};

use crate::{bob::Bob, server::init_server};


static RUNTIME: OnceLock<Runtime> = OnceLock::new();

pub fn ensure_runtime() -> &'static Runtime {
    RUNTIME.get_or_init(|| {
        tokio::runtime::Builder::new_multi_thread()
            .thread_name("eteru-tokio")
            // .worker_threads(2) 
            // .thread_stack_size(3 * 1024 * 1024)
            // Essential: Enable IO and Time drivers
            .enable_all() 
            .build()
            .expect("Tokio runtime init failed")
    })
}

pub fn setup_axum(http_port: u16) -> JoinHandle<Result<(), Error>> {
    let rt = ensure_runtime();
    let handle = rt.spawn(init_server(http_port));

    handle
}


pub fn reqwest() -> JoinHandle<Result<String>> {
    let rt = ensure_runtime();
    let handle = rt.spawn( reqwest_job());
    
    handle
}

pub fn reqwest_result(h: JoinHandle<Result<String>>) -> Result<String> {
    let rt = ensure_runtime();

    rt.block_on(h.into_future())?
}

async fn reqwest_job() -> Result<String> {
    let res = reqwest::get("http://localhost:9009").await?; // "https://httpbin.org/ip"
        
    if res.status().as_u16() > 299 {
        bail!("I guess, the request failed")
    }

    // Err("some error message")
    let tres = res.text().await?;

    Ok(tres)
}

pub fn tungstenite() -> Result<String> {
    let rt = ensure_runtime();
    rt.block_on(_tungstenite())
}

async fn _tungstenite() -> Result<String> {
    let rt = ensure_runtime();
    let (ws, _) = tokio_tungstenite::connect_async("ws://localhost:9009/ws").await?;
    
    let (mut write, mut read) = ws.split();
    let (channel_tx, mut channel_rx) = mpsc::channel::<Message>(32);

    let _ = write.send(Message::Text(Utf8Bytes::from_static("greetings from client")));

    let mut send_task = rt.spawn(async move {
        while let Some(msg) = channel_rx.recv().await {
            let should_terminate = match msg {
                Message::Close(_) => true,
                _ => false
            };

            if write
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

    let mut recv_task = rt.spawn(async move {
        while let Some(Ok(msg)) = read.next().await {
            // print message and break if instructed to do so
            
            let (do_break, msg): (bool, Option<Message>) = match process_message(msg) {
                ControlFlow::Break(reply) => (true, reply),
                ControlFlow::Continue(reply) => (false, reply)
            };

            if let Some(reply) = msg {
                if channel_tx.send(reply).await.is_err() {
                    break;
                }
            }
            if do_break { break; }
        }
    });

    tokio::select! {
        rv_a = (&mut send_task) => {
            match rv_a {
                Ok(()) => println!("sender socket terminated"),
                Err(b) => println!("Error sending messages {b:?}")
            }
            recv_task.abort();
        },
        rv_b = (&mut recv_task) => {
            match rv_b {
                Ok(()) => println!("reciever socket terminated"),
                Err(b) => println!("Error receiving messages {b:?}")
            }
            send_task.abort();
        }
    }


    Ok(String::from("return"))
}



fn process_message(msg: Message) -> ControlFlow<Option<Message>, Option<Message>> {
    match msg {
        Message::Text(t) => {
            println!(">>> received str: {t:?}");
            let msg = Message::Text(Utf8Bytes::from_static("we heard yo"));
            return ControlFlow::Break(Some(msg));
        }
        Message::Binary(d) => { 
            // INFO: file should be within 65535 byte including header.
            println!(">>> received {} bytes: {d:?}", d.len());
        }
        Message::Close(c) => {
            if let Some(cf) = c {
                println!(
                    ">>> received close with code {} and reason `{}`",
                    cf.code, cf.reason
                );
            } else {
                println!(">>>received close message without CloseFrame");
            }
            return ControlFlow::Break(None);
        }
        Message::Pong(_) => {}
        Message::Ping(_) => { /* we don't touch this. */ }
        _ => { /* we don't touch this. */ }
    }
    ControlFlow::Continue(None)
}


const TABLE: TableDefinition<&str, &[u8]> = TableDefinition::new("my_data");

pub fn redb() -> Result<()> {
    let db = Database::create("my_db.redb")?;
    let bob = Bob::new(13480, String::from("hello darkness"));
    let write_txn = db.begin_write()?;
    {
        let bobb = bob.to_blob()?;
        let bob_s = bobb.as_ref();
        let mut table = write_txn.open_table(TABLE)?;
        table.insert("my_key", bob_s)?;
    }
    write_txn.commit()?;

    let read_txn = db.begin_read()?;
    let table = read_txn.open_table(TABLE)?;
    let table_q= table.get("my_key")?.unwrap();
    let mut v = AlignedVec::<16>::new();
    v.extend_from_slice(table_q.value());
    
    println!("Bob seems happy");
    let bob_d = Bob::from_blob(v)?;
    println!("serialize test with bob : {:?}", bob_d);
    

    Ok(())
}
