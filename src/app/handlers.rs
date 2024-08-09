use std::ops::Deref;

use hyper::{Body, Response};
use routerify::RequestInfo;
use serde_json::json;

use crate::json_response;
use crate::misc::{
    AppError, AppResult, HttpRequest, HttpResponse, not_found, ok_response, Params, StringExt,
};
use crate::model::{CreateParams, DestroyParams, JoinParams, LastAnnouncementParams};
use crate::service::{ChatService, Room};

pub async fn default_handler(req: HttpRequest) -> AppResult<HttpResponse> {
    let uri = req.uri();
    let action = uri.path().substring_after_last('/').to_string();
    let room = uri.path().substring_before_last('/').to_string();
    #[cfg(debug_assertions)]
    println!("handle: {uri}");
    match action.as_str() {
        "" => not_found(),
        "create" => {
            if room.is_empty() {
                not_found()
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

fn app(req: &HttpRequest) -> &ChatService {
    use routerify::prelude::RequestExt;
    req.data::<ChatService>().unwrap()
}

/// matches /path/to/room/create
async fn create_room(req: HttpRequest, room: String, params: CreateParams) -> AppResult<HttpResponse> {
    app(&req).tx.CreateRoom(Room {
        uid: room,
        secret: params.secret,
        post: params.post,
        post_types: params.post_types,
    }).await?;
    Ok(ok_response())
}

/// matches /status
async fn dump_status(req: HttpRequest) -> AppResult<HttpResponse> {
    let status = app(&req).tx.Status().await;
    Ok(json_response!(status))
}

/// matches /path/to/room/action
async fn room_action(req: HttpRequest, room: String, action: String) -> AppResult<HttpResponse> {
    let chat_room = app(&req).tx.GetRoom(room).await?;

    match action.as_str() {
        "join" => {
            let params = JoinParams::parse_uri(req.uri())?;
            chat_room.join(req, params).await
        }
        "destroy" => {
            let params = DestroyParams::parse_uri(req.uri())?;
            if &params.secret == chat_room.secret.deref() {
                chat_room.send.spawn().Destroy();
                Ok(ok_response())
            } else {
                Err(AppError::secret())
            }
        }
        "count" => {
            Ok(json_response!({
                "count": chat_room.send.Count().await,
            }))
        }
        "status" => {
            Ok(json_response!(chat_room.send.Status().await))
        }
        "lastAnnouncement" => {
            let params = LastAnnouncementParams::parse_uri(req.uri())?;
            let announcements = chat_room.send.LastAnnouncement(params.types).await;
            Ok(json_response!(announcements))
        }
        "participants" => {
            let participants = chat_room.send.Participants().await;
            Ok(json_response!(participants))
        }
        "messages" => {
            let messages = chat_room.send.Messages().await;
            Ok(json_response(messages))
        }
        _ => not_found()
    }
}