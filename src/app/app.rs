use std::collections::HashMap;
use tokio::sync::mpsc::{channel, Sender};
use tokio::sync::oneshot;
use crate::app::Room;
use crate::app::room::RoomInfo;
use crate::error;
use crate::misc::{Responder, Result};
use crate::model::CreateParams;

pub struct App {
    tx: Sender<Command>,
}

enum Command {
    CreateRoom {
        room: String,
        params: CreateParams,
        resp_tx: Responder<()>,
    },
    Status {
        resp_tx: Responder<Vec<RoomInfo>>,
    },
    GetRoom {
        room: String,
        resp_tx: Responder<RoomInfo>,
    },
}

#[derive(Default)]
struct AppState {
    rooms: HashMap<String, Room>,
}

impl App {
    pub fn create() -> App {
        let (tx, mut rx) = channel::<Command>(30);
        let app = App { tx };

        tokio::spawn(async move {
            use Command::*;
            let mut app_state = AppState::default();

            while let Some(command) = rx.recv().await {
                match command {
                    CreateRoom { room, params, resp_tx } => {
                        let result = app_state.create_room(room, params);
                        resp_tx.send(result).unwrap();
                    }
                    Status { resp_tx } => {
                        let result = app_state.status();
                        resp_tx.send(result).unwrap();
                    }
                    GetRoom { room, resp_tx } => {
                        let result = app_state.get_room(&room);
                        resp_tx.send(result).unwrap();
                    }
                }
            }
        });

        app
    }

    pub async fn create_room(&self, room: String, params: CreateParams) -> Result<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.clone().send(Command::CreateRoom { room, params, resp_tx }).await.unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn status(&self) -> Result<Vec<RoomInfo>> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.clone().send(Command::Status { resp_tx }).await.unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn get_room(&self, room: &str) -> Result<RoomInfo> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let command = Command::GetRoom { room: room.to_string(), resp_tx };
        self.tx.send(command).await.unwrap();
        resp_rx.await.unwrap()
    }
}

impl AppState {
    fn status(&self) -> Result<Vec<RoomInfo>> {
        Ok(self.rooms.values().map(|e| e.status()).collect())
    }

    fn create_room(&mut self, room: String, params: CreateParams) -> Result<()> {
        if self.rooms.contains_key(&room) {
            error!("Room is not available".to_string())
        } else {
            let instance = Room::builder()
                .room(room.to_string())
                .secret(params.secret)
                .post(params.post)
                .post_types(params.post_types)
                .build();

            self.rooms.insert(room, instance);
            Ok(())
        }
    }

    fn get_room(&self, room: &str) -> Result<RoomInfo> {
        match self.rooms.get(room) {
            None => error!(hyper::StatusCode::NOT_FOUND, "Room not found".to_string()),
            Some(value) => Ok(value.status())
        }
    }
}