use std::string::ToString;

use http_body_util::Full;
use hyper::{Request, Response, StatusCode};
use hyper::body::{Bytes, Incoming};

pub use app_error::{AppError, ToBadRequest};
pub use option_ext::OptionExt;
pub use query_params::{Params, QueryParams};
pub use response::*;
pub use string_ext::StringExt;

pub mod date_serde;

mod string_ext;
mod app_error;
mod query_params;
mod response;
mod option_ext;
mod command;

pub type AppResult<T> = Result<T, AppError>;

pub type HttpRequest = Request<Incoming>;

pub type HttpResponse = Response<Full<Bytes>>;

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
