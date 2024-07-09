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

impl AppError {
    pub fn bad_request(message: String) -> AppError {
        AppError {
            code: StatusCode::BAD_REQUEST,
            message: Some(message),
        }
    }

    pub fn secret() -> AppError {
        AppError {
            code: StatusCode::UNAUTHORIZED,
            message: Some("Secret does not match".to_string()),
        }
    }
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

pub trait ToBadRequest<T> {
    fn to_bad_request(self) -> Result<T, AppError>;
}

impl<T, E> ToBadRequest<T> for Result<T, E> where E: Display {
    fn to_bad_request(self) -> Result<T, AppError> {
        self.map_err(|e| AppError::bad_request(format!("{e}")))
    }
}