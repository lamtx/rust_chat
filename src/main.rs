use std::net::SocketAddr;

use hyper::Server;
use routerify::{Router, RouterService};

use crate::app::handlers;
use crate::misc::*;
use crate::service::ChatService;

mod model;
mod misc;
#[macro_use]
mod app;
mod service;

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .data(ChatService::create())
        .any(handlers::default_handler)
        .err_handler_with_info(handlers::error_handler)
        .build()
        .unwrap();

    // Create a Service from the router above to handle incoming requests.
    let service = RouterService::new(router).unwrap();

    // The address on which the server will be listening.
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));

    // Create a server by passing the created service to `.serve` method.
    let server = Server::bind(&addr).serve(service);

    println!("App is running on: {addr}");
    if let Err(err) = server.await {
        eprintln!("Server error: {err}");
    }
}
