use std::collections::HashMap;
use tokio::sync::{mpsc, oneshot};
use crate::app::{Room, RoomParams};
use crate::app::room::RoomInfo;
use crate::error;
use crate::misc::{not_found, AppResult};
use crate::model::CreateParams;

pub struct App {
    tx: mpsc::Sender<Command>,
}

enum Command {
    CreateRoom {
        room: String,
        params: CreateParams,
        resp_tx: oneshot::Sender<AppResult<()>>,
    },
    Status {
        resp_tx: oneshot::Sender<Vec<RoomInfo>>,
    },
    GetRoom {
        room: String,
        resp_tx: oneshot::Sender<AppResult<Room>>,
    },
}

#[derive(Default)]
struct AppImpl {
    rooms: HashMap<String, Room>,
}

impl App {
    pub fn create() -> App {
        let (tx, mut rx) = mpsc::channel::<Command>(30);
        let app = App { tx };

        tokio::spawn(async move {
            use Command::*;
            let mut app_state = AppImpl::default();

            while let Some(command) = rx.recv().await {
                match command {
                    CreateRoom { room, params, resp_tx } => {
                        let result = app_state.create_room(room, params);
                        resp_tx.send(result).unwrap();
                    }
                    Status { resp_tx } => {
                        let result = app_state.status().await;
                        resp_tx.send(result).unwrap();
                    }
                    GetRoom { room, resp_tx } => {
                        let result = app_state.get_room(&room);
                        resp_tx.send(result).unwrap_or_else(|_| panic!("channel broken"))
                    }
                }
            }
        });

        app
    }

    pub async fn create_room(&self, room: String, params: CreateParams) -> AppResult<()> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send(Command::CreateRoom { room, params, resp_tx }).await.unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn status(&self) -> Vec<RoomInfo> {
        let (resp_tx, resp_rx) = oneshot::channel();
        self.tx.send(Command::Status { resp_tx }).await.unwrap();
        resp_rx.await.unwrap()
    }

    pub async fn get_room(&self, room: &str) -> AppResult<Room> {
        let (resp_tx, resp_rx) = oneshot::channel();
        let command = Command::GetRoom { room: room.to_string(), resp_tx };
        self.tx.send(command).await.unwrap();
        resp_rx.await.unwrap()
    }
}

impl AppImpl {
    async fn status(&self) -> Vec<RoomInfo> {
        let mut result = Vec::new();
        for e in self.rooms.values() {
            result.push(e.status().await)
        }
        result
    }

    fn create_room(&mut self, room: String, params: CreateParams) -> AppResult<()> {
        if self.rooms.contains_key(&room) {
            error!("Room is not available".to_string())
        } else {
            let instance = RoomParams {
                room: room.to_string(),
                secret: params.secret,
                post: params.post,
                post_types: params.post_types,
            }.create();
            self.rooms.insert(room, instance);
            Ok(())
        }
    }

    fn get_room(&self, room: &str) -> AppResult<Room> {
        if let Some(instance) = self.rooms.get(room) {
            Ok(instance.clone())
        } else {
            not_found()
        }
    }
}