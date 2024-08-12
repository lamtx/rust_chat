use http_body_util::Full;
use hyper::{Response, StatusCode};
use hyper::body::Bytes;
use hyper::header::CONTENT_TYPE;

use crate::misc::HttpResponse;

pub fn empty_body() -> Full<Bytes> {
    // completely does not understand but it comes from
    // https://hyper.rs/guides/1/server/echo/
    Full::new(const { Bytes::new() })
}

pub fn string_body(text: String) -> Full<Bytes> {
    // completely does not understand but it comes from
    // https://hyper.rs/guides/1/server/echo/
    Full::new(Bytes::from(text))
}

pub fn ok_response() -> HttpResponse {
    Response::builder()
        .status(StatusCode::OK)
        .body(empty_body())
        .unwrap()
}

pub fn json_response(json: String) -> HttpResponse {
    Response::builder()
        .status(StatusCode::OK)
        .header(CONTENT_TYPE, "application/json")
        // again, completely does not understand but it comes from
        // https://hyper.rs/guides/1/server/echo/
        .body(string_body(json))
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