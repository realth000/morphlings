use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Resource {
    pub file_path: PathBuf,
}

#[derive(Debug)]
pub enum PlayerEvent {
    Started(Resource),
    Paused(Resource),
    Resumed(Resource),
    ErrorOccured(Box<dyn std::error::Error>),
    Finished(Resource),
}

#[derive(Debug)]
pub enum PlayerCommand {
    Pause,
    Resume,
    ChangeVolume(f32),
    ChangeVolumeTo(f32),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub resources: Vec<PathBuf>,
    pub player: PlayerConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerConfig {
    pub volume: f32,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum HttpApiCode {
    Success = 0,
    FailedToSendPlayerCommand = 1,
    InvalidRequestParameter = 2,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct HttpApiResponse {
    pub code: HttpApiCode,
    pub message: String,
    pub data: Option<serde_json::Value>,
}
