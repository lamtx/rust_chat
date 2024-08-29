use std::net::SocketAddr;

use axum::{Json, Router};
use axum::extract::{Path, Query, State, WebSocketUpgrade};
use axum::response::IntoResponse;
use axum::routing::{get, post};
use tokio::net::TcpListener;

use crate::config::PORT;
use crate::misc::*;
use crate::model::{CreateParams, JoinParams, Room, RoomInfo};
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
    let app = Router::new()
        .route("/create/*room", post(create))
        .route("/join/*room", get(join))
        .route("/status", get(status))
        .route("/status/*room", get(room_status))
        .with_state(ChatService::create());
    let listener = TcpListener::bind(addr).await.unwrap();
    println!("socket opened");
    axum::serve(listener, app).await.unwrap();
}

async fn create(
    State(service): State<ChatService>,
    Path(room): Path<String>,
    Query(params): Query<CreateParams>,
) {
    service
        .op
        .CreateRoom(Room {
            uid: room,
            secret: params.secret,
            post: params.post,
            post_types: params.post_types,
        })
        .await
        .unwrap();
}
async fn join(
    State(service): State<ChatService>,
    ws: WebSocketUpgrade,
    Path(room): Path<String>,
    Query(params): Query<JoinParams>,
) -> impl IntoResponse {
    let room = service.op.GetRoom(room).await.unwrap();
    room.join(ws, params).await
}

async fn status(State(service): State<ChatService>) -> Json<Vec<RoomInfo>> {
    Json(service.op.Status().await)
}

async fn room_status(
    State(service): State<ChatService>,
    Path(room): Path<String>,
) -> Json<RoomInfo> {
    let room = service.op.GetRoom(room).await.unwrap();
    Json(room.op.Status().await)
}
