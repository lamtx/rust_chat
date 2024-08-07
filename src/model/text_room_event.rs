use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "textroom")]
pub enum TextRoomEvent {
    #[serde(rename = "announcement")]
    Announcement {
        #[serde(with = "crate::misc::date_serde")]
        date: DateTime<Utc>,
        text: String,
        r#type: String,
    },

    #[serde(rename = "banned")]
    Banned,

    #[serde(rename = "destroyed")]
    Destroyed,

    #[serde(rename = "join")]
    Joined {
        username: Option<String>,
        display: Option<String>,
        participants: usize,
    },

    #[serde(rename = "leave")]
    Left {
        username: Option<String>,
        display: Option<String>,
        participants: usize,
    },

    #[serde(rename = "message")]
    Message {
        from: String,
        display: String,
        #[serde(with = "crate::misc::date_serde")]
        date: DateTime<Utc>,
        text: String,
        r#type: String,
    },
}

pub struct TextRoom;
