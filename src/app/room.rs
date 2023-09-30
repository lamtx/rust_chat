use serde::Serialize;
use crate::app::client::Client;
use crate::misc::{HttpRequest, HttpResponse, Result};
use crate::model::{Message, Participant};

#[derive(Default)]
pub struct Room {
    room: String,
    secret: String,
    post: Option<String>,
    post_types: Option<Vec<String>>,
    inner: Inner,
}

#[derive(Default)]
struct Inner {
    messages: Vec<Message>,
    participants: Vec<Client>,
}

#[derive(Default)]
pub struct Builder {
    pub room: String,
    pub secret: String,
    pub post: Option<String>,
    pub post_types: Option<Vec<String>>,
}

#[derive(Debug, Serialize)]
pub struct RoomInfo {
    pub room: String,
    pub participants: Vec<Participant>,
    pub messages: usize,
}

impl Room {
    #[inline]
    pub fn builder() -> Builder {
        Builder::default()
    }

    pub async fn handle_request(&self, req: &HttpRequest, action: &str) -> Result<HttpResponse> {
        todo!()
    }

    pub fn post_created(&self) {}

    pub fn status(&self) -> RoomInfo {
        RoomInfo {
            room: self.room.to_string(),
            participants: self.inner.participants
                .iter().map(|e| e.me.clone())
                .collect(),
            messages: self.inner.messages.len(),
        }
    }
}

impl Builder {
    pub fn room(mut self, room: String) -> Self {
        self.room = room;
        self
    }
    pub fn secret(mut self, secret: String) -> Self {
        self.secret = secret;
        self
    }
    pub fn post(mut self, post: Option<String>) -> Self {
        self.post = post;
        self
    }
    pub fn post_types(mut self, post_types: Option<Vec<String>>) -> Self {
        self.post_types = post_types;
        self
    }

    pub fn build(self) -> Room {
        Room {
            room: self.room,
            secret: self.secret,
            post: self.post,
            post_types: self.post_types,
            inner: Inner::default(),
        }
    }
}