use serde::Serialize;

#[derive(Serialize)]
pub enum TextRoomResponse {
    Ok {
        transaction: Option<String>,
        ok: String,
    },
    Error {
        transaction: Option<String>,
        message: String,
    },
}

