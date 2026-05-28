use std::{env, path::PathBuf};

use morphlings_apis::{Config, PlayerCommand, PlayerEvent, PlayerState};
use snafu::{ResultExt, Snafu};
use tokio::sync::{broadcast, watch};

use crate::{
    http::{HttpError, start_http_server},
    player::{PlayerError, start_player},
    resource::scan_resource,
};

mod http;
mod player;
mod resource;

#[derive(Debug, Snafu)]
enum Error {
    #[snafu(display("server config file path not set"))]
    ConfigPathNotSet,

    #[snafu(display("failed to read {file_type} {path}: {source}"))]
    FailedToReadFile {
        source: std::io::Error,
        file_type: &'static str,
        path: String,
    },

    #[snafu(display("invalid config: {source}"))]
    InvalidConfig { source: serde_json::Error },

    #[snafu(display("failed to scan resource: {source}"))]
    FailedToScanResource { source: Box<Error> },

    #[snafu(display("error occured in player thread: {source}"))]
    PlayerThreadErrored { source: PlayerError },

    #[snafu(display("error occured in http server thread: {source}"))]
    HttpThreadErrored { source: HttpError },

    #[snafu(whatever, display("other error: {message}"))]
    Whatever {
        message: String,
        #[snafu(source(from(Box<dyn std::error::Error>, Some)))]
        source: Option<Box<dyn std::error::Error>>,
    },
}

type ServerResult<T> = std::result::Result<T, Error>;

async fn run() -> ServerResult<()> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        // No config specified.
        return Err(ConfigPathNotSetSnafu.build());
    }

    let config_path = PathBuf::from(args.get(1).unwrap());

    let config_data = std::fs::read(&config_path).context(FailedToReadFileSnafu {
        file_type: "config",
        path: args.get(1).unwrap(),
    })?;

    let config =
        serde_json::from_slice::<Config>(config_data.as_slice()).context(InvalidConfigSnafu)?;

    let mut all_resources = vec![];

    if config.resources.is_empty() {
        println!("no resource directory set in config");
    } else {
        for resource_dir in config.resources {
            all_resources.extend(
                scan_resource(resource_dir)
                    .map_err(Box::new)
                    .context(FailedToScanResourceSnafu)?,
            );
        }
    }

    println!("all resources count: {}", all_resources.len());

    let (player_command_tx, player_command_rx) = broadcast::channel::<PlayerCommand>(1);
    let (player_event_tx, _player_event_rx) = broadcast::channel::<PlayerEvent>(1);
    let (player_state_tx, player_state_rx) = watch::channel::<PlayerState>(PlayerState::default());

    tokio::select!(
        player_error = start_player(
            all_resources,
            config.player,
            player_event_tx,
            player_command_rx,
            player_state_tx,
        ) => player_error.context(PlayerThreadErroredSnafu),
        http_error = start_http_server(player_command_tx, player_state_rx) => http_error.context(HttpThreadErroredSnafu),
    )
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }
}
