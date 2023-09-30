use std::ops::Deref;
use crate::app::App;
use crate::error;
use crate::misc::{Result, HttpRequest, HttpResponse, ToListResponse, ok_response, StringExt, Params, AppError};
use crate::model::CreateParams;
use hyper::{Body, Response, StatusCode};
use routerify::RequestInfo;

pub async fn default_handler(req: HttpRequest) -> Result<HttpResponse> {
    let uri = req.uri();
    let action = uri.path().substring_after_last('/').to_string();
    let room = uri.path().substring_before_last('/').to_string();
    #[cfg(debug_assertions)]
    println!("handle: {uri}");
    match action.as_str() {
        "" => error!(StatusCode::NOT_FOUND, "Not found".to_string()),
        "create" => {
            if room.is_empty() {
                error!(StatusCode::NOT_FOUND, "Not found".to_string())
            } else {
                let params = Params::parse_uri(uri)?;
                create_room(req, room, params).await
            }
        }
        "status" => {
            if room.is_empty() {
                dump_status(req).await
            } else {
                room_action(req, room, action).await
            }
        }
        &_ => {
            room_action(req, room, action).await
        }
    }
}

pub async fn error_handler(err: routerify::RouteError, _: RequestInfo) -> HttpResponse {
    let error = err.downcast::<AppError>().unwrap();
    let status_code = error.code;
    let json = serde_json::to_string(error.deref()).unwrap();
    #[cfg(debug_assertions)]
    println!("Error {status_code}: {json}");
    Response::builder()
        .status(status_code)
        .body(Body::from(json))
        .unwrap()
}

fn app(req: &HttpRequest) -> &App {
    use routerify::prelude::RequestExt;
    req.data::<App>().unwrap()
}

/// matches /path/to/room/create
async fn create_room(req: HttpRequest, room: String, params: CreateParams) -> Result<HttpResponse> {
    app(&req).create_room(room, params).await?;
    Ok(ok_response())
}

/// matches /status
async fn dump_status(req: HttpRequest) -> Result<HttpResponse> {
    let status = app(&req).status().await?;
    Ok(status.to_response())
}

/// matches /path/to/room/action
async fn room_action(req: HttpRequest, room: String, action: String) -> Result<HttpResponse> {
    todo!()
}