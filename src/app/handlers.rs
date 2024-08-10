use std::convert::Infallible;
use std::ops::Deref;

use http_body_util::Full;
use hyper::Response;
use serde_json::json;

use crate::json_response;
use crate::misc::{AppError, AppResult, HttpRequest, HttpResponse, not_found, ok_response, Params, StringExt, ToBadRequest};
use crate::model::{CreateParams, DestroyParams, JoinParams, LastAnnouncementParams};
use crate::service::{ChatService, Room};

pub async fn default_handler(service: &ChatService, req: HttpRequest) -> Result<HttpResponse, Infallible> {
    match handle_request(service, req).await {
        Ok(res) => Ok(res),
        Err(error) => {
            let status_code = error.code;
            let json = serde_json::to_string(&error).unwrap();
            #[cfg(debug_assertions)]
            println!("Error {status_code}: {json}");
            Ok(Response::builder()
                .status(status_code)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(Full::new(json.into()))
                .unwrap())
        }
    }
}

pub async fn handle_request(service: &ChatService, req: HttpRequest) -> AppResult<HttpResponse> {
    let uri = req.uri();
    let action = uri.path().substring_after_last('/').to_string();
    let room = uri.path().substring_before_last('/').to_string();
    #[cfg(debug_assertions)]
    println!("{}: {uri}", req.method());
    match action.as_str() {
        "" => not_found(),
        "create" => {
            if room.is_empty() {
                not_found()
            } else {
                let params = Params::parse_uri(uri)?;
                create_room(service, req, room, params).await
            }
        }
        "status" => {
            if room.is_empty() {
                dump_status(service, req).await
            } else {
                room_action(service, req, room, action).await
            }
        }
        &_ => {
            room_action(service, req, room, action).await
        }
    }
}

/// matches /path/to/room/create
async fn create_room(
    service: &ChatService,
    _: HttpRequest,
    room: String,
    params: CreateParams,
) -> AppResult<HttpResponse> {
    service.op.CreateRoom(Room {
        uid: room,
        secret: params.secret,
        post: params.post,
        post_types: params.post_types,
    }).await?;
    Ok(ok_response())
}

/// matches /status
async fn dump_status(service: &ChatService, _: HttpRequest) -> AppResult<HttpResponse> {
    let status = service.op.Status().await;
    Ok(json_response!(status))
}

/// matches /path/to/room/action
async fn room_action(
    service: &ChatService,
    req: HttpRequest,
    room: String,
    action: String,
) -> AppResult<HttpResponse> {
    let chat_room = service.op.GetRoom(room).await?;

    match action.as_str() {
        "join" => {
            let params = JoinParams::parse_uri(req.uri())?;
            chat_room.join(req, params).await.to_bad_request()
        }
        "destroy" => {
            let params = DestroyParams::parse_uri(req.uri())?;
            if &params.secret == chat_room.secret.deref() {
                chat_room.op.spawn().Destroy();
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
            Ok(json_response!(chat_room.op.Status().await))
        }
        "lastAnnouncement" => {
            let params = LastAnnouncementParams::parse_uri(req.uri())?;
            let announcements = chat_room.op.LastAnnouncement(params.types).await;
            Ok(json_response!(announcements))
        }
        "participants" => {
            let participants = chat_room.op.Participants().await;
            Ok(json_response!(participants))
        }
        "messages" => {
            let messages = chat_room.op.Messages().await;
            Ok(json_response(messages))
        }
        _ => not_found()
    }
}