//! Async event system — merges terminal input, ticks, and docker refresh
//! into a single mpsc channel consumed by the main loop.

use std::time::Duration;

use crossterm::event::{Event as CtEvent, EventStream, KeyEvent};
use futures::StreamExt;
use tokio::sync::mpsc;

use crate::docker;

/// All events the main loop processes.
#[derive(Debug)]
pub enum AppEvent {
    /// A key was pressed.
    Key(KeyEvent),
    /// Terminal was resized (ratatui handles it on next draw).
    Resize,
    /// Periodic tick for animations and message expiry (200ms).
    Tick,
    /// Background docker status refresh completed.
    DockerRefresh(docker::RefreshResult),
    /// A docker operation (start/stop/rebuild/etc.) completed.
    OpComplete { success: bool, message: String },
}

pub struct EventLoop {
    rx: mpsc::UnboundedReceiver<AppEvent>,
    tx: mpsc::UnboundedSender<AppEvent>,
}

impl EventLoop {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();

        // ── Task 1: Terminal input events ───────────────────────────
        let tx_input = tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            while let Some(Ok(event)) = reader.next().await {
                let app_event = match event {
                    CtEvent::Key(key) => Some(AppEvent::Key(key)),
                    CtEvent::Resize(_, _) => Some(AppEvent::Resize),
                    _ => None,
                };
                if let Some(e) = app_event {
                    if tx_input.send(e).is_err() {
                        break;
                    }
                }
            }
        });

        // ── Task 2: Render tick (200ms) ─────────────────────────────
        let tx_tick = tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(200));
            loop {
                interval.tick().await;
                if tx_tick.send(AppEvent::Tick).is_err() {
                    break;
                }
            }
        });

        // ── Task 3: Docker status refresh (every 5s) ───────────────
        let tx_docker = tx.clone();
        tokio::spawn(async move {
            // Small initial delay so the first frame renders before blocking on docker
            tokio::time::sleep(Duration::from_millis(300)).await;
            loop {
                let result = docker::refresh_all().await;
                if tx_docker.send(AppEvent::DockerRefresh(result)).is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_secs(5)).await;
            }
        });

        Self { rx, tx }
    }

    /// Get the next event. Returns None when all senders are dropped.
    pub async fn next(&mut self) -> Option<AppEvent> {
        self.rx.recv().await
    }

    /// Clone the sender so background tasks can push events back.
    pub fn sender(&self) -> mpsc::UnboundedSender<AppEvent> {
        self.tx.clone()
    }
}
