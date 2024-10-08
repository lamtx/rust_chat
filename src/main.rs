use std::net::SocketAddr;
use std::sync::Arc;

use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper_util::rt::TokioIo;
use tokio::net::TcpListener;

use crate::app::handlers::default_handler;
use crate::config::PORT;
use crate::misc::*;
use crate::service::ChatService;

mod misc;
mod model;
#[macro_use]
mod app;
mod config;
mod service;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    log!("warn: DEBUG mode");
    let addr = SocketAddr::from(([0, 0, 0, 0], PORT));
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("service is running on: http://{addr}");

    let global_state = Arc::new(ChatService::create());
    loop {
        let (stream, _) = listener.accept().await.unwrap();

        // Use an adapter to access something implementing `tokio::io` traits as if they implement
        // `hyper::rt` IO traits.
        let io = TokioIo::new(stream);
        let service = global_state.clone();
        // Spawn a tokio task to serve multiple connections concurrently
        tokio::spawn(async move {
            if let Err(err) = http1::Builder::new()
                .serve_connection(io, service_fn(|req| default_handler(&service, req)))
                // Support WS upgradable protocol
                .with_upgrades()
                .await
            {
                log!("Error serving connection: {:?}", err);
            }
        });
    }
}
