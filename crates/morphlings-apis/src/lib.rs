use std::path::PathBuf;

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
}
