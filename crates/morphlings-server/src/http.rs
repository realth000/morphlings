use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::{get, post},
};
use morphlings_apis::{HttpApiCode, HttpApiResponse, PlayMode, PlayerCommand};
use serde::{Deserialize, Serialize};
use snafu::Snafu;
use tokio::sync::broadcast::{Sender, error::SendError};

#[derive(Debug, Deserialize, Serialize)]
struct ApiResponse(HttpApiResponse);

impl ApiResponse {
    fn new(code: HttpApiCode, message: String) -> Self {
        Self(HttpApiResponse {
            code,
            message,
            data: None,
        })
    }

    fn new_with_data(data: serde_json::Value) -> Self {
        Self(HttpApiResponse {
            code: HttpApiCode::Success,
            message: "success".into(),
            data: Some(data),
        })
    }

    fn new_success() -> Self {
        Self(HttpApiResponse {
            code: HttpApiCode::Success,
            message: "success".into(),
            data: None,
        })
    }
}

impl IntoResponse for ApiResponse {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0.code {
            HttpApiCode::Success => StatusCode::OK,
            HttpApiCode::FailedToSendPlayerCommand => StatusCode::INTERNAL_SERVER_ERROR,
            HttpApiCode::InvalidRequestParameter => StatusCode::BAD_REQUEST,
        };

        let body = Json(self.0);

        (status, body).into_response()
    }
}

impl From<SendError<PlayerCommand>> for ApiResponse {
    fn from(value: SendError<PlayerCommand>) -> Self {
        Self::new(
            HttpApiCode::FailedToSendPlayerCommand,
            format!("failed to send player command: {:?}", value),
        )
    }
}

#[derive(Debug, Snafu)]
pub(super) enum HttpError {
    #[snafu(display("failed to launch http server"))]
    FailedToLaunch,
}

struct AppState {
    player_command_tx: Sender<PlayerCommand>,
}

fn send_player_command(state: Arc<AppState>, player_command: PlayerCommand) -> ApiResponse {
    match state.player_command_tx.send(player_command) {
        Ok(_) => ApiResponse::new_success(),
        Err(e) => e.into(),
    }
}

async fn on_get_root() -> Html<String> {
    let page_data = include_str!("html/index.html");
    Html(page_data.into())
}

async fn on_get_pause(State(state): State<Arc<AppState>>) -> ApiResponse {
    send_player_command(state, PlayerCommand::Pause)
}

async fn on_get_resume(State(state): State<Arc<AppState>>) -> ApiResponse {
    send_player_command(state, PlayerCommand::Resume)
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct VolumeParams {
    value: Option<f32>,
    delta: Option<f32>,
}

async fn on_post_volume(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<VolumeParams>,
) -> ApiResponse {
    if let Some(value) = payload.value {
        send_player_command(state, PlayerCommand::ChangeVolumeTo(value))
    } else if let Some(delta) = payload.delta {
        send_player_command(state, PlayerCommand::ChangeVolume(delta))
    } else {
        ApiResponse::new(
            HttpApiCode::InvalidRequestParameter,
            "missing delta or value field".into(),
        )
    }
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct PlayModeParams {
    mode: PlayMode,
}

async fn on_post_play_mode(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<PlayModeParams>,
) -> ApiResponse {
    send_player_command(state, PlayerCommand::SetPlayMode(payload.mode))
}

async fn on_post_play_previous(State(state): State<Arc<AppState>>) -> ApiResponse {
    send_player_command(state, PlayerCommand::PlayPrevious)
}

async fn on_post_play_next(State(state): State<Arc<AppState>>) -> ApiResponse {
    send_player_command(state, PlayerCommand::PlayNext)
}

pub(super) async fn start_http_server(
    player_command_tx: Sender<PlayerCommand>,
) -> Result<(), HttpError> {
    let shared_state = Arc::new(AppState { player_command_tx });

    let app = Router::new()
        .route("/", get(on_get_root))
        .route("/pause", get(on_get_pause))
        .route("/resume", get(on_get_resume))
        .route("/volume", post(on_post_volume))
        .route("/playMode", post(on_post_play_mode))
        .route("/playPrevious", post(on_post_play_previous))
        .route("/playNext", post(on_post_play_next))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9012").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
