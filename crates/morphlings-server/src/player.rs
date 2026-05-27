use rand::RngExt;
use std::{fs::File, io::BufReader};
use tokio::sync::broadcast::{Receiver, Sender};

use morphlings_apis::{PlayMode, PlayerCommand, PlayerConfig, PlayerEvent, Resource};
use rodio::{DeviceSinkError, Player as RodioPlayer};
use snafu::{ResultExt, Snafu};

#[derive(Debug, Snafu)]
pub(super) enum PlayerError {
    #[snafu(display("failed to open device sink: {source}"))]
    FailedToOpenDeviceSink { source: DeviceSinkError },
}

pub struct PlayerManager {
    resources: Vec<Resource>,
    rodio_player: RodioPlayer,
    curr_resource_index: Option<usize>,
    player_event_tx: Sender<PlayerEvent>,
    player_command_rx: Receiver<PlayerCommand>,
    config: PlayerConfig,

    /// List of resources index played in history.
    ///
    /// The most recent played resource is at the tail of history.
    ///
    /// Basically this field is only used in shuffle mode, because in other play mode
    /// we have stable next/prev order, but keep it for "play history" feature.
    play_history: Vec<usize>,

    /// The index of resource in [Self::play_history].
    ///
    /// Use this index to control move next and previous.
    ///
    /// This field is only used in shuffle mode.
    ///
    /// When go previous in shuffle mode, we should move the curor index back by 1,
    /// and if then go next again, advance the cursor index by 1 instead of playing
    /// a new random resource.
    shuffle_mode_play_history_index: usize,
}

impl PlayerManager {
    async fn run(&mut self) {
        self.apply_config_on_init();

        let mut ticker = tokio::time::interval(tokio::time::Duration::from_millis(200));
        // Start!
        if let Some(index) = self.curr_resource_index {
            println!("[player] initial play at index {index}");
            self.play_resource_at_index(index);
        }

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    if self.rodio_player.empty() && self.curr_resource_index.is_some() {
                        println!("[player] finished");
                        self.send_player_event(PlayerEvent::Finished(self.resources.get(self.curr_resource_index.unwrap()).unwrap().clone()));
                        self.play_next();
                    }
                },
                Ok(cmd) = self.player_command_rx.recv() => {
                    println!("[player] recvive command: {cmd:?}");

                    match cmd {
                        PlayerCommand::Pause => self.rodio_player.pause(),
                        PlayerCommand::Resume => self.rodio_player.play(),
                        PlayerCommand::ChangeVolume(delta) => self.rodio_player.set_volume(self.rodio_player.volume() + delta),
                        PlayerCommand::ChangeVolumeTo(value) => self.rodio_player.set_volume(value),
                        PlayerCommand::SetPlayMode(play_mode) => self.change_play_mode(play_mode),
                        PlayerCommand::PlayNext => self.play_next(),
                        PlayerCommand::PlayPrevious => self.play_previous(),
                    }
                }
            }
        }
    }

    fn send_player_event(&mut self, player_event: PlayerEvent) {
        if let Err(e) = self.player_event_tx.send(player_event) {
            println!("[player] failed to send player event: {e:?}");
        }
    }

    fn apply_config_on_init(&self) {
        self.rodio_player.set_volume(self.config.volume);
    }

    /// Play previous one.
    fn play_previous(&mut self) {
        match self.config.play_mode {
            PlayMode::Default | PlayMode::RepeatList | PlayMode::RepeatTrack => {
                // Use the previous one in play history.
                match self.curr_resource_index {
                    Some(0) => {
                        let next_index = if self.config.play_mode == PlayMode::Default {
                            // Reach the head of playlist, repeat!
                            0
                        } else {
                            // Reach the head of playlist, go to the last one.
                            self.resources.len() - 1
                        };

                        self.curr_resource_index = Some(next_index);
                        self.play_resource_at_index(next_index);
                    }
                    Some(v) => {
                        self.curr_resource_index = Some(v - 1);
                        self.play_resource_at_index(v - 1);
                    }
                    None => {
                        self.play_resource_at_index(0);
                    }
                }
            }
            PlayMode::Shuffle => {
                // Use the previous one in play history.
                if self.shuffle_mode_play_history_index > 0 {
                    self.shuffle_mode_play_history_index -= 1;
                    self.play_resource_in_history_at_index(self.shuffle_mode_play_history_index);
                } else {
                    // Repeat the current one if we reached the head of play history.
                    self.play_resource_in_history_at_index(self.shuffle_mode_play_history_index);
                }
            }
        }
    }

    /// Play next according to the current [PlayMode].
    fn play_next(&mut self) {
        match self.config.play_mode {
            PlayMode::Default | PlayMode::RepeatList => {
                if self.resources.is_empty() {
                    self.stop();
                    return;
                }

                // Default mode, play the next resource if any, or stop if reached the end of resouces.
                let current_index = self.curr_resource_index.unwrap_or(0);

                let next_index = if current_index < self.resources.len() - 1 {
                    current_index + 1
                } else {
                    // Reach the end of resources.
                    match self.config.play_mode {
                        PlayMode::Default => {
                            // Repeat the last one.
                            self.resources.len() - 1
                        }
                        PlayMode::RepeatList => {
                            // Go back to the first one.
                            0
                        }
                        PlayMode::RepeatTrack | PlayMode::Shuffle => {
                            unreachable!("mode not going to be here")
                        }
                    }
                };

                self.curr_resource_index = Some(next_index);
                if !self.play_resource_at_index(next_index) {
                    self.stop();
                }
            }
            PlayMode::RepeatTrack => {
                let try_repeat_result = self
                    .curr_resource_index
                    .or_else(|| Some(0))
                    .map(|x| self.play_resource_at_index(x));
                if try_repeat_result != Some(true) {
                    self.stop();
                }
            }
            PlayMode::Shuffle => {
                // Play random resource.
                if self.resources.is_empty() {
                    return;
                }

                if self.resources.len() == 1 {
                    self.shuffle_mode_play_history_index = 0;
                    self.play_resource_in_history_at_index(0);
                    return;
                }

                if self.shuffle_mode_play_history_index < self.play_history.len() - 1 {
                    // We are in the middle of play history, get the next one in play history.
                    self.shuffle_mode_play_history_index += 1;
                    self.play_resource_in_history_at_index(self.shuffle_mode_play_history_index);
                } else {
                    // We are at the end of shuffle history, get next random one.
                    let mut rng = rand::rng();
                    let next_index =
                        rng.sample(rand::distr::Uniform::new(0, self.resources.len()).unwrap());
                    if !self.play_resource_at_index(next_index) {
                        // Unreachable?
                        self.stop();
                    }
                };
            }
        }
    }

    /// Play the resource at [index].
    ///
    /// Return false if resource index is out of range.
    fn play_resource_at_index(&mut self, index: usize) -> bool {
        if self.play_history.last() != Some(&index) {
            // Record to the play history if index is not duplicated with the latest one.
            self.play_history.push(index);

            if self.config.play_mode == PlayMode::Shuffle {
                // Update shuffle mode index.
                self.shuffle_mode_play_history_index = self.play_history.len() - 1;
            }
        }

        match self.resources.get(index) {
            Some(resource) => {
                self.play_resource(resource);
                true
            }
            None => {
                println!("[player] failed to play resource at index {index}: index out of range");
                false
            }
        }
    }

    /// Play the resource at index in [Self::play_history].
    ///
    /// Does not change [Self::play_history] and [Self::shuffle_mode_play_history_index];
    ///
    /// **The caller MUST ensure [index] if not out of range**
    fn play_resource_in_history_at_index(&self, history_index: usize) {
        match self
            .play_history
            .get(history_index)
            .and_then(|x| self.resources.get(*x))
        {
            Some(resource) => {
                self.play_resource(resource);
            }
            None => {
                println!(
                    "[player] failed to play resource in play history at history index {history_index}: index out of range"
                );
            }
        }
    }

    /// Do not use this function directly.
    ///
    /// Use [Self::play_resource_at_index] instead.
    fn play_resource(&self, resource: &Resource) {
        let file = BufReader::new(File::open(&resource.file_path).unwrap());
        let audio = rodio::Decoder::try_from(file).unwrap();
        self.rodio_player.stop();
        self.rodio_player.clear();
        self.rodio_player.append(audio);
        self.rodio_player.play();
    }

    fn stop(&mut self) {
        self.rodio_player.stop();
        self.rodio_player.clear();
        self.curr_resource_index = None;
        self.send_player_event(PlayerEvent::Stopped);
    }

    fn change_play_mode(&mut self, play_mode: PlayMode) {
        if self.config.play_mode == play_mode {
            return;
        }

        self.play_history.clear();
        self.config.play_mode = play_mode;
    }
}

pub(super) async fn start_player(
    resources: Vec<Resource>,
    config: PlayerConfig,
    player_event_tx: Sender<PlayerEvent>,
    player_command_rx: Receiver<PlayerCommand>,
) -> Result<(), PlayerError> {
    let handle =
        rodio::DeviceSinkBuilder::open_default_sink().context(FailedToOpenDeviceSinkSnafu)?;

    let mut player_manager = PlayerManager {
        resources,
        rodio_player: rodio::Player::connect_new(&handle.mixer()),
        curr_resource_index: Some(0),
        player_event_tx,
        player_command_rx,
        config,
        play_history: vec![],
        shuffle_mode_play_history_index: 0,
    };

    player_manager.run().await;

    Ok(())
}
