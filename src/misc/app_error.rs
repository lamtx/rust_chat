use std::error::Error;
use std::fmt::{Display, Formatter};
use hyper::StatusCode;
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct AppError {
    #[serde(skip_serializing)]
    pub code: StatusCode,
    pub message: Option<String>,
}

impl Display for AppError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} - {}", self.code, match &self.message {
            None => "null",
            Some(msg) => &msg,
        })
    }
}

impl Error for AppError {}