use std::time::Duration;

use camino::Utf8PathBuf;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Resource {
    pub file_path: Utf8PathBuf,
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
    pub resources: Vec<Utf8PathBuf>,
    pub player: PlayerConfig,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct PlayerConfig {
    pub volume: f32,
    pub play_mode: PlayMode,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub enum PlayState {
    /// Stopped.
    ///
    /// Resourced may be loaded or not.
    Stopped,

    /// Playing an audio.
    Playing,

    /// Paused when playing an audio.
    Paused,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PlayerState {
    /// The audio currently playing.
    pub current_resource: Option<Resource>,

    /// Current play state.
    pub play_state: PlayState,

    /// Volume.
    ///
    /// Value between 0 and 1.
    pub volume: f32,

    /// Current play mode.
    pub play_mode: PlayMode,

    /// Current duration in audio playing.
    pub duration: Duration,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            current_resource: None,
            play_state: PlayState::Stopped,
            volume: 0.0,
            play_mode: PlayMode::Default,
            duration: Duration::ZERO,
        }
    }
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
