use std::{fs::File, io::BufReader};
use tokio::sync::mpsc::{Receiver, Sender};

use morphlings_apis::{PlayerCommand, PlayerEvent, Resource};
use rodio::{DeviceSinkError, Player as RodioPlayer};
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub(super) enum PlayerError {
    #[snafu(display("failed to open device sink: {source}"))]
    FailedToOpenDeviceSink { source: DeviceSinkError },
}

pub struct PlayerManager {
    rodio_player: RodioPlayer,
    curr_resource: Resource,
    player_event_tx: Sender<PlayerEvent>,
    player_command_rx: Receiver<PlayerCommand>,
}

impl PlayerManager {
    async fn run(&mut self) {
        let mut ticker = tokio::time::interval(tokio::time::Duration::from_millis(200));
        let file = BufReader::new(File::open(&self.curr_resource.file_path).unwrap());
        let audio = rodio::Decoder::try_from(file).unwrap();
        self.rodio_player.append(audio);

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if self.rodio_player.empty() {
                        // Empty.
                        println!(">>> player finished");
                        return;
                    }
                },
                Some(cmd) = self.player_command_rx.recv() => {
                    match cmd {
                        PlayerCommand::Pause => self.rodio_player.pause(),
                        PlayerCommand::Resume => self.rodio_player.play(),
                    }
                    println!(">>> recvive command: {cmd:?}");
                }
            }
        }
    }
}

pub(super) async fn start_player(
    resource: &Resource,
    player_event_tx: Sender<PlayerEvent>,
    player_command_rx: Receiver<PlayerCommand>,
) -> Result<(), PlayerError> {
    let handle =
        rodio::DeviceSinkBuilder::open_default_sink().context(FailedToOpenDeviceSinkSnafu)?;

    let rodio_player = rodio::Player::connect_new(&handle.mixer());

    let mut player_manager = PlayerManager {
        rodio_player,
        curr_resource: resource.clone(),
        player_event_tx,
        player_command_rx,
    };

    player_manager.run().await;

    Ok(())
}
