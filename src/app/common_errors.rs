use std::fmt::Display;

use hyper::StatusCode;

use crate::app::app_error::{AppError, ToBadRequest};
use crate::misc::ParseParamError;
use crate::service::ServiceError;

#[macro_export]
macro_rules! error {
	($status_code: expr, $message: expr) => {{
        Err(AppError {
            code: $status_code,
            message: Some($message),
        })
	}};
	($message: expr) => {
		{
		   Err(AppError {
            code: hyper::StatusCode::BAD_REQUEST,
            message: Some($message),
        })
		}
	}
}

#[inline]
pub fn error<T>(message: String) -> Result<T, AppError> {
    Err(AppError::bad_request(message))
}

#[macro_export]
macro_rules! not_found {
	($message: expr) => {{
        Err(AppError {
            code: hyper::StatusCode::NOT_FOUND,
            message: Some($message),
        })
    }}
}

#[inline]
pub fn not_found<T>() -> Result<T, AppError> {
    Err(AppError {
        code: StatusCode::NOT_FOUND,
        message: Some("Not found".to_string()),
    })
}

impl<T> ToBadRequest<T> for Result<T, ServiceError> {
    fn to_bad_request(self) -> Result<T, AppError> {
        self.map_err(|e| match e {
            ServiceError::RoomNotFound => AppError::not_found("Room not found".to_string())
        })
    }
}

impl<T> ToBadRequest<T> for Result<T, ParseParamError<'static>> {
    fn to_bad_request(self) -> Result<T, AppError> {
        self.map_err(|e| match e {
            ParseParamError::FieldRequired { name } => AppError::bad_request(format!("{name} is required."))
        })
    }
}

impl<T, E> ToBadRequest<T> for Result<T, E>
where
    E: Display,
{
    fn to_bad_request(self) -> Result<T, AppError> {
        self.map_err(|e| AppError::bad_request(format!("{e}")))
    }
}
