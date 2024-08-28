use std::collections::HashMap;
use std::ops::Deref;

use crate::command;
use crate::model::{Room, RoomInfo};
use crate::service::{ChatRoom, ServiceError};

command! {
    pub CreateRoom(room: Room) -> Result<(), ServiceError>;
    pub Status()  -> Vec<RoomInfo>;
    pub GetRoom(room: String) -> Result<ChatRoom, ServiceError>;
    pub DestroyRoom(room: String, secret: String) -> Result<(), ServiceError>;
}

pub struct ChatService {
    pub op: CommandSender,
}

impl ChatService {
    pub fn create() -> ChatService {
        let (op, mut rx) = Command::new_channel();
        let app = ChatService { op: op.clone() };
        tokio::spawn(async move {
            use Command::*;
            let mut state = ChatServiceInner::default();

            while let Some(command) = rx.recv().await {
                match command {
                    CreateRoom { room, resp_tx } => {
                        let _ = resp_tx.send(state.create_room(room));
                    }
                    Status { resp_tx } => {
                        let _ = resp_tx.send(state.status().await);
                    }
                    GetRoom { room, resp_tx } => {
                        let _ = resp_tx.send(state.get_room(&room));
                    }
                    DestroyRoom {
                        room,
                        secret,
                        resp_tx,
                    } => {
                        let _ = resp_tx.send(state.destroy_room(room, secret));
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
            result.push(e.op.Status().await)
        }
        result
    }

    fn create_room(&mut self, room: Room) -> Result<(), ServiceError> {
        if self.rooms.contains_key(&room.uid) {
            Err(ServiceError::RoomNotFound)
        } else {
            let uid = room.uid.clone();
            let chat_room = ChatRoom::create(room);
            self.rooms.insert(uid, chat_room);
            Ok(())
        }
    }

    fn get_room(&self, room: &str) -> Result<ChatRoom, ServiceError> {
        if let Some(instance) = self.rooms.get(room) {
            Ok(instance.clone())
        } else {
            Err(ServiceError::RoomNotFound)
        }
    }

    fn destroy_room(&mut self, uid: String, secret: String) -> Result<(), ServiceError> {
        if let Some(room) = self.rooms.get(&uid) {
            if room.secret.deref() == &secret {
                room.op.spawn().Destroy();
                self.rooms.remove(&uid);
                Ok(())
            } else {
                Err(ServiceError::SecretNotMatch)
            }
        } else {
            Err(ServiceError::RoomNotFound)
        }
    }
}
