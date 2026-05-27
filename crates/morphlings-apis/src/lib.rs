use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone)]
pub struct Resource {
    pub file_path: PathBuf,
}

#[derive(Debug, Clone)]
pub enum PlayerEvent {
    Started(Resource),
    Paused(Resource),
    Resumed(Resource),
    ErrorOccured(String),
    Finished(Resource),
    Stopped,
}

#[derive(Debug, Clone)]
pub enum PlayerCommand {
    Pause,
    Resume,
    PlayNext,
    PlayPrevious,
    ChangeVolume(f32),
    ChangeVolumeTo(f32),
    SetPlayMode(PlayMode),
}

#[derive(Debug, Deserialize, Serialize, Clone, Default, PartialEq, Eq)]
pub enum PlayMode {
    #[default]
    Default,
    RepeatList,
    RepeatTrack,
    Shuffle,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Config {
    pub resources: Vec<PathBuf>,
    pub player: PlayerConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerConfig {
    pub volume: f32,
    pub play_mode: PlayMode,
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
