use std::sync::Arc;

use axum::{Router, extract::State, routing::get};
use morphlings_apis::PlayerCommand;
use snafu::Snafu;
use tokio::sync::mpsc::Sender;

#[derive(Debug, Snafu)]
pub(super) enum HttpError {
    #[snafu(display("failed to launch http server"))]
    FailedToLaunch,
}

struct AppState {
    player_command_tx: Sender<PlayerCommand>,
}

async fn on_get_pause(State(state): State<Arc<AppState>>) -> String {
    match state.player_command_tx.send(PlayerCommand::Pause).await {
        Ok(_) => "paused".into(),
        Err(e) => format!(">>> on_get_pause: {e:?}"),
    }
}

async fn on_get_resume(State(state): State<Arc<AppState>>) -> String {
    match state.player_command_tx.send(PlayerCommand::Resume).await {
        Ok(_) => "resumed".into(),
        Err(e) => format!(">>> on_get_resume: {e:?}"),
    }
}

pub(super) async fn start_http_server(
    player_command_tx: Sender<PlayerCommand>,
) -> Result<(), HttpError> {
    let shared_state = Arc::new(AppState { player_command_tx });

    let app = Router::new()
        .route("/pause", get(on_get_pause))
        .route("/resume", get(on_get_resume))
        .with_state(shared_state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:9012").await.unwrap();
    axum::serve(listener, app).await.unwrap();

    Ok(())
}
