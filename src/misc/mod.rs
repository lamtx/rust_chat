pub mod date_serde;

use std::string::ToString;
use hyper::StatusCode;
pub use string_ext::StringExt;
pub use option_ext::OptionExt;
pub use app_error::{AppError, ToBadRequest};
pub use query_params::{Params, QueryParams, Get, TryGet};

pub use response::*;

mod string_ext;
mod app_error;
mod query_params;
mod response;
mod option_ext;

pub type AppResult<T> = Result<T, AppError>;

pub type HttpRequest = hyper::http::Request<hyper::body::Body>;

pub type HttpResponse = hyper::http::Response<hyper::body::Body>;

#[inline]
pub fn error<T>(message: String) -> AppResult<T> {
    Err(AppError::bad_request(message))
}

#[inline]
pub fn not_found<T>() -> AppResult<T> {
    Err(AppError {
        code: StatusCode::NOT_FOUND,
        message: Some("Not found".to_string()),
    })
}

#[macro_export]
macro_rules! error {
	($a: expr, $b: expr) => {{
        Err(crate::misc::AppError {
            code: $a,
            message: Some($b),
        })
	}};
	($a: expr) => {
		{
		   Err(crate::misc::AppError {
            code: hyper::StatusCode::BAD_REQUEST,
            message: Some($a),
        })
		}
	}
}
