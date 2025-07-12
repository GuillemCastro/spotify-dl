use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use librespot::core::Session;
use librespot::playback::config::{Bitrate, PlayerConfig};
use librespot::playback::mixer::NoOpVolume;
use librespot::playback::player::{Player, PlayerEvent};
use tokio::sync::mpsc::UnboundedSender;

use crate::stream::channel_sink::{ChannelSink, SinkEvent};
use crate::stream::{StreamError, StreamEvent, StreamEventChannel};
use crate::track::Track;

pub struct Stream {
    player_config: PlayerConfig,
    session: Session,
}

impl Stream {
    pub fn new(session: Session) -> Self {
        let config = PlayerConfig {
            bitrate: Bitrate::Bitrate320,
            ..Default::default()
        };
        Stream {
            player_config: config,
            session,
        }
    }

    pub async fn stream(&self, track: Track) -> Result<StreamEventChannel> {
        let metadata = track.metadata(&self.session).await?;
        let (sink, mut channel) = ChannelSink::new(metadata);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();

        let player = Player::new(
            self.player_config.clone(),
            self.session.clone(),
            Box::new(NoOpVolume),
            move || Box::new(sink),
        );

        tokio::spawn(async move {
            match tryhard::retry_fn(|| async { Self::load(player.clone(), &track).await })
                .retries(3)
                .on_retry(|attempt, _, e| {
                    let error = format!("{}", e);
                    let tx = tx.clone();
                    async move {
                        tracing::warn!(
                            "Attempt {} to load track {:?} failed: {}",
                            attempt,
                            track.id,
                            error
                        );
                        Self::send_event(&tx, StreamEvent::Retry {
                            attempt: attempt as usize,
                            max_attempts: 3,
                        }).await;
                    }
                })
                .exponential_backoff(Duration::from_secs(10))
                .max_delay(Duration::from_secs(30))
                .await
            {
                Ok(_) => tracing::info!("Track loaded successfully: {:?}", track.id),
                Err(e) => {
                    tracing::error!("Failed to load track: {:?}, error: {:?}", track.id, e);
                    Self::send_event(
                        &tx,
                        StreamEvent::Error(StreamError::LoadError(format!(
                            "Failed to load track: {:?}",
                            track.id
                        ))),
                    )
                    .await;
                    return;
                }
            }

            tracing::info!("Streaming track: {:?}", track.id);

            while let Some(event) = channel.recv().await {
                match event {
                    SinkEvent::Write {
                        bytes,
                        total,
                        content,
                    } => {
                        Self::send_event(
                            &tx,
                            StreamEvent::Write {
                                bytes,
                                total,
                                content,
                            },
                        )
                        .await
                    }
                    SinkEvent::Finished => {
                        Self::send_event(&tx, StreamEvent::Finished).await;
                        break;
                    }
                }
            }
        });

        Ok(rx)
    }

    async fn load(player: Arc<Player>, track: &Track) -> Result<()> {
        player.load(track.id, true, 0);

        tracing::info!("Loading track: {:?}", track.id);
        loop {
            match player.get_player_event_channel().recv().await {
                Some(PlayerEvent::Playing { .. })
                | Some(PlayerEvent::TrackChanged { .. })
                | Some(PlayerEvent::EndOfTrack { .. }) => {
                    tracing::info!("Player started playing track: {:?}", track.id);
                    break;
                }
                Some(PlayerEvent::Unavailable { .. }) => {
                    tracing::info!("Track is unavailable: {:?}", track.id);
                    return Err(anyhow::anyhow!("Could not load track: {:?}", track.id));
                }
                _ => {
                    // Ignore other events
                }
            }
        }

        tokio::spawn(async move {
            player.await_end_of_track().await;
            player.stop();
        });

        Ok(())
    }

    async fn send_event(tx: &UnboundedSender<StreamEvent>, event: StreamEvent) {
        tx.send(event).unwrap_or_else(|e| {
            tracing::error!("Failed to send event: {:?}", e);
        });
    }
}
