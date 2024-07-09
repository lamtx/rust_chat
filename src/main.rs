mod model;
mod misc;
#[macro_use]
mod app;

use hyper::{Body, Response, Server, StatusCode};
use routerify::{Router, RouterService, RequestInfo};
use std::net::SocketAddr;
use std::ops::Deref;
use tokio::task::yield_now;

use crate::app::{App, handlers};
use crate::misc::{*};

#[tokio::main]
async fn main() {
    let router = Router::builder()
        .data(App::create())
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
