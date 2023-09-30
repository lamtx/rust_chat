pub mod date_serde;

use hyper::StatusCode;
pub use string_ext::StringExt;
pub use app_error::AppError;
pub use query_params::{Params, QueryParams, Get, TryGet};

pub use response::*;

mod string_ext;
mod app_error;
mod query_params;
mod response;

pub type Result<T> = std::result::Result<T, AppError>;

pub type HttpRequest = hyper::http::Request<hyper::body::Body>;

pub type HttpResponse = hyper::http::Response<hyper::body::Body>;

/// Provided by the requester and used by the manager task to send
/// the command response back to the requester.
pub type Responder<T> = tokio::sync::oneshot::Sender<Result<T>>;

#[inline]
pub fn error<T>(message: String) -> Result<T> {
    Err(AppError {
        code: StatusCode::BAD_REQUEST,
        message: Some(message),
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