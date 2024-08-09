use std::collections::HashMap;

use tokio::sync::{mpsc};

use crate::{command, error};
use crate::misc::{AppResult, not_found};
use crate::model::{RoomInfo};
use crate::service::{ChatRoom, Room};

command! {
    pub CreateRoom(room: Room) -> AppResult<()>,
    pub Status()  -> Vec<RoomInfo>,
    pub GetRoom(room: String) -> AppResult<ChatRoom>,
}

pub struct ChatService {
    pub tx: CommandSender,
}

impl ChatService {
    pub fn create() -> ChatService {
        let (tx, mut rx) = mpsc::channel::<Command>(30);
        let app = ChatService { tx: CommandSender { tx } };
        tokio::spawn(async move {
            use Command::*;
            let mut app_state = ChatServiceInner::default();

            while let Some(command) = rx.recv().await {
                match command {
                    CreateRoom { room, resp_tx } => {
                        let result = app_state.create_room(room);
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
}

#[derive(Default)]
struct ChatServiceInner {
    rooms: HashMap<String, ChatRoom>,
}

impl ChatServiceInner {
    async fn status(&self) -> Vec<RoomInfo> {
        let mut result = Vec::new();
        for e in self.rooms.values() {
            result.push(e.tx.Status().await)
        }
        result
    }

    fn create_room(&mut self, room: Room) -> AppResult<()> {
        if self.rooms.contains_key(&room.uid) {
            error!("Room is not available".to_string())
        } else {
            let uid = room.uid.clone();
            self.rooms.insert(uid, ChatRoom::create(room));
            Ok(())
        }
    }

    fn get_room(&self, room: &str) -> AppResult<ChatRoom> {
        if let Some(instance) = self.rooms.get(room) {
            Ok(instance.clone())
        } else {
            not_found()
        }
    }
}
