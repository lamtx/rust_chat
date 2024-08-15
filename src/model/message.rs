use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Clone, Serialize)]
pub struct Message<'a> {
    pub textroom: &'static str,
    pub room: &'a str,
    pub r#type: &'a str,
    pub from: &'a str,
    pub text: &'a str,
    #[serde(with = "crate::misc::date_serde")]
    pub date: DateTime<Utc>,
}

impl<'a> Message<'a> {
    pub const MODERATE: &'static str = "moderate";
    pub const ANNOUNCEMENT: &'static str = "announcement";
    pub const MESSAGE: &'static str = "message";
    pub const TYPE_BAN: &'static str = "ban";
    pub const TYPE_ROOM_CREATED: &'static str = "room_created";
    pub const TYPE_ROOM_DESTROYED: &'static str = "room_destroyed";

    pub fn room_created(room: &'a str) -> Message<'a> {
        Message {
            textroom: Message::MODERATE,
            room,
            r#type: Message::TYPE_ROOM_CREATED.into(),
            text: "",
            date: Utc::now(),
            from: "",
        }
    }

    pub fn room_destroyed(room: &'a str) -> Message<'a> {
        Message {
            textroom: Message::MODERATE,
            room,
            r#type: Message::TYPE_ROOM_DESTROYED.into(),
            text: "",
            date: Utc::now(),
            from: "",
        }
    }
}
