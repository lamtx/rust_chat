use std::convert::Infallible;

use http_body_util::Full;
use hyper::Response;
use serde_json::json;
use tokio::runtime::Handle;

use crate::{json_response, log};
use crate::app::app_error::{AppError, ToBadRequest};
use crate::app::common_errors::not_found;
use crate::misc::{empty_body, HttpRequest, HttpResponse, ok_response, Params, StringExt};
use crate::model::{
    CreateParams, DestroyParams, JoinParams, LastAnnouncementParams, PhotoParams, Room,
};
use crate::service::ChatService;

pub async fn default_handler(
    service: &ChatService,
    req: HttpRequest,
) -> Result<HttpResponse, Infallible> {
    match handle_request(service, req).await {
        Ok(res) => Ok(res),
        Err(error) => {
            let status_code = error.code;
            let json = serde_json::to_string(&error).unwrap();
            log!("Err {status_code}: {json}");
            Ok(Response::builder()
                .status(status_code)
                .header(hyper::header::CONTENT_TYPE, "application/json")
                .body(Full::new(json.into()))
                .unwrap())
        }
    }
}

pub async fn handle_request(
    service: &ChatService,
    req: HttpRequest,
) -> Result<HttpResponse, AppError> {
    let uri = req.uri();
    let action = uri.path().substring_after_last('/').to_string();
    let room = uri.path().substring_before_last('/').to_string();
    log!("{}: {uri}", req.method());
    match action.as_str() {
        "" => not_found(),
        "create" => {
            if room.is_empty() {
                not_found()
            } else {
                let params = Params::parse_uri(uri).to_bad_request()?;
                create_room(service, req, room, params).await
            }
        }
        "destroy" => {
            if room.is_empty() {
                not_found()
            } else {
                let params = DestroyParams::parse_uri(req.uri()).to_bad_request()?;
                destroy_room(service, req, room, params).await
            }
        }
        "status" => {
            if room.is_empty() {
                Ok(dump_status(service, req).await)
            } else {
                room_action(service, req, room, action).await
            }
        }
        "debug" => {
            if room.is_empty() {
                let metrics = Handle::current().metrics().num_alive_tasks();

                Ok(json_response!({
                    "tasks": metrics
                }))
            } else {
                not_found()
            }
        }
        &_ => room_action(service, req, room, action).await,
    }
}

/// matches /path/to/room/create
async fn create_room(
    service: &ChatService,
    _: HttpRequest,
    room: String,
    params: CreateParams,
) -> Result<HttpResponse, AppError> {
    service
        .op
        .CreateRoom(Room {
            uid: room,
            secret: params.secret,
            post: params.post,
            post_types: params.post_types,
        })
        .await
        .to_bad_request()?;
    Ok(ok_response())
}

/// matches /path/to/room/destroy
async fn destroy_room(
    service: &ChatService,
    _: HttpRequest,
    room: String,
    params: DestroyParams,
) -> Result<HttpResponse, AppError> {
    service
        .op
        .DestroyRoom(room, params.secret)
        .await
        .to_bad_request()?;
    Ok(ok_response())
}
/// matches /status
async fn dump_status(service: &ChatService, _: HttpRequest) -> HttpResponse {
    let status = service.op.Status().await;
    json_response!(status)
}

/// matches /path/to/room/other_action
async fn room_action(
    service: &ChatService,
    req: HttpRequest,
    room: String,
    action: String,
) -> Result<HttpResponse, AppError> {
    let chat_room = service.op.GetRoom(room).await.to_bad_request()?;

    match action.as_str() {
        "join" => {
            let params = JoinParams::parse_uri(req.uri()).to_bad_request()?;
            chat_room.join(req, params).await.to_bad_request()
        }
        "count" => Ok(json_response!({
            "count": chat_room.op.Count().await,
        })),
        "status" => Ok(json_response!(chat_room.op.Status().await)),
        "lastAnnouncement" => {
            let params = LastAnnouncementParams::parse_uri(req.uri()).to_bad_request()?;
            let announcements = chat_room.op.LastAnnouncement(params.types).await;
            Ok(json_response!(announcements))
        }
        "participants" => {
            let participants = chat_room.op.Participants().await;
            Ok(json_response!(participants))
        }
        "photo" => {
            let params = PhotoParams::parse_uri(req.uri()).to_bad_request()?;
            let photo = chat_room.op.Photo(params.username).await;
            match photo {
                None => not_found(),
                Some(url) => Ok(Response::builder()
                    .status(hyper::StatusCode::FOUND)
                    .header(hyper::header::LOCATION, url)
                    .body(empty_body())
                    .unwrap()),
            }
        }
        _ => not_found(),
    }
}
