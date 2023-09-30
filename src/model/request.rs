pub enum TextRoomRequest {
    Announcement {
        r#type: String,
        text: String,
        secret: String,
    },

    Ban {
        username: String,
        secret: String,
    },

    Leave,

    Message {
        r#type: String,
        text: String,
    },
}