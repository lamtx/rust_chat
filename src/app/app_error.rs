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
    pub fn not_found(message: String) -> AppError {
        AppError {
            code: StatusCode::NOT_FOUND,
            message: Some(message),
        }
    }
}

pub trait ToBadRequest<T> {
    fn to_bad_request(self) -> Result<T, AppError>;
}
