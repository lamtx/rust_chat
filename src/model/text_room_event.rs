use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Eq, PartialEq)]
#[serde(tag = "textroom")]
pub enum TextRoomEvent<'a> {
    #[serde(rename = "announcement")]
    Announcement {
        #[serde(with = "crate::misc::date_serde")]
        date: DateTime<Utc>,
        text: &'a str,
        r#type: &'a str,
    },

    #[serde(rename = "banned")]
    Banned,

    #[serde(rename = "destroyed")]
    Destroyed,

    #[serde(rename = "join")]
    Joined {
        username: Option<&'a str>,
        display: Option<&'a str>,
        participants: usize,
    },

    #[serde(rename = "leave")]
    Left {
        username: Option<&'a str>,
        display: Option<&'a str>,
        participants: usize,
    },

    #[serde(rename = "message")]
    Message {
        from: &'a str,
        display: &'a str,
        #[serde(with = "crate::misc::date_serde")]
        date: DateTime<Utc>,
        text: &'a str,
        r#type: &'a str,
    },
}