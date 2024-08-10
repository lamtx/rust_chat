use http_body_util::Full;
use hyper::{Response, StatusCode};
use hyper::body::Bytes;
use hyper::header::CONTENT_TYPE;

use crate::misc::HttpResponse;

pub fn ok_response() -> HttpResponse {
    Response::builder()
        .status(StatusCode::OK)
        // completely does not understand but it comes from
        // https://hyper.rs/guides/1/server/echo/
        .body(Full::new(const { Bytes::new() }))
        .unwrap()
}

pub fn json_response(json: String) -> HttpResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        // again, completely does not understand but it comes from
        // https://hyper.rs/guides/1/server/echo/
        .body(Full::new(json.into()))
        .unwrap()
}

#[macro_export]
macro_rules! json_response {
    ($($json:tt)+) => {{
        let value = json!($($json)+);
        let json_body = serde_json::to_string(&value).unwrap();
        json_response(json_body)
    }};
}