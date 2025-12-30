use std::{thread, time::Duration};

pub mod config;
pub mod bob;
pub mod count;
pub mod handle;
pub mod server;

fn main() {
    let result = config::Configuration::from_file("config-template.toml");
    
    let cfg = match result {
        Ok(cfg) => cfg,
        Err(e) => {
            println!("thrown: {:?}", e);
            return;
        }
    };

    println!("config had server {}", cfg.local.server_enable);
    println!("config points to {} servers", cfg.servers.len());

    
    let _ = handle::setup_axum(cfg.local.port);
    thread::sleep(Duration::from_secs(1));

    let handle = handle::reqwest();
    let result = handle::reqwest_result(handle);
    match result {
        Ok(r) => println!("received : {r}"),
        Err(e) => println!("thrown: {:?}", e)
    }

    let result = handle::tungstenite();
    match result {
        Ok(_) => {},
        Err(e) => {
            let e = e.root_cause();
            println!("thrown: {:?}", e);
        }
    }

    let result = count::mmap_blak3();
    match result {
        Ok(hash) => println!("hash was : {hash}"),
        Err(e) => println!("thrown: {:?}", e)
    }

    let result = handle::redb();
    match result {
        Ok(_) => {},
        Err(e) => {
            let e= e.root_cause();
            panic!("db failed miserably: {:?}", e);
        }
    }
    

    thread::sleep(Duration::from_secs(5));
}
