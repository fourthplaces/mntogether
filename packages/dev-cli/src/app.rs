//! Application state, mode management, key handling, and operation dispatch.

use std::collections::HashMap;
use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use tokio::sync::mpsc;

use crate::docker;
use crate::events::AppEvent;
use crate::services::{self, Layer, ServiceId};

// ── Status types ────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Status {
    Ok,
    Starting,
    Fail,
    Stopped,
}

#[derive(Debug, Clone)]
pub struct ServiceState {
    pub status: Status,
    pub cpu: Option<String>,
    pub hint: Option<String>,
}

impl Default for ServiceState {
    fn default() -> Self {
        Self {
            status: Status::Stopped,
            cpu: None,
            hint: None,
        }
    }
}

// ── Menu types ──────────────────────────────────────────────────────

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuTarget {
    Layer(Layer),
    All,
}

impl MenuTarget {
    pub fn label(self) -> &'static str {
        match self {
            MenuTarget::Layer(l) => l.label(),
            MenuTarget::All => "All Services",
        }
    }

    pub fn has_rebuild(self) -> bool {
        match self {
            MenuTarget::Layer(l) => l.has_rebuild(),
            MenuTarget::All => true,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    Main,
    LayerAction(MenuTarget),
}

// ── App ─────────────────────────────────────────────────────────────

pub struct App {
    pub mode: Mode,
    pub states: HashMap<ServiceId, ServiceState>,
    pub action_msg: Option<(String, Instant)>,
    pub pending_op: Option<(String, Instant)>,
    pub should_quit: bool,
    pub wants_logs: Option<Vec<String>>,
    pub repo_root: String,
}

impl App {
    pub fn new(repo_root: String) -> Self {
        Self {
            mode: Mode::Main,
            states: HashMap::new(),
            action_msg: None,
            pending_op: None,
            should_quit: false,
            wants_logs: None,
            repo_root,
        }
    }

    /// Apply a docker refresh result into cached state.
    pub fn apply_refresh(&mut self, result: docker::RefreshResult) {
        self.states = result.states;
    }

    /// Called on every 200ms tick — expires old messages.
    pub fn tick(&mut self) {
        if let Some((_, created)) = &self.action_msg {
            if created.elapsed().as_secs() >= 15 {
                self.action_msg = None;
            }
        }
    }

    /// Called when a background docker operation completes.
    pub fn complete_op(&mut self, success: bool, message: String) {
        self.pending_op = None;
        let prefix = if success { "✓" } else { "✗" };
        self.action_msg = Some((format!("{prefix} {message}"), Instant::now()));
    }

    // ── Key handling ────────────────────────────────────────────────

    pub fn handle_key(&mut self, key: KeyEvent, tx: &mpsc::UnboundedSender<AppEvent>) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        // Ctrl+C quits from anywhere
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        match self.mode {
            Mode::Main => self.handle_main_key(key, tx),
            Mode::LayerAction(target) => self.handle_submenu_key(key, target, tx),
        }
    }

    fn handle_main_key(&mut self, key: KeyEvent, tx: &mpsc::UnboundedSender<AppEvent>) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                self.should_quit = true;
            }
            KeyCode::Char('i') => {
                self.mode = Mode::LayerAction(MenuTarget::Layer(Layer::Infra));
            }
            KeyCode::Char('b') => {
                self.mode = Mode::LayerAction(MenuTarget::Layer(Layer::Backend));
            }
            KeyCode::Char('f') => {
                self.mode = Mode::LayerAction(MenuTarget::Layer(Layer::Frontend));
            }
            KeyCode::Char('a') => {
                self.mode = Mode::LayerAction(MenuTarget::All);
            }
            KeyCode::Char(c @ '1'..='9') => {
                let idx = (c as u8 - b'1') as usize;
                if let Some(svc) = services::url_services().get(idx) {
                    if let Some(url) = svc.url {
                        open_url(url);
                    }
                }
            }
            KeyCode::Char('d') | KeyCode::Char('D') => {
                self.spawn_op("Resetting database", tx, |root| async move {
                    docker::reset_database(&root).await?;
                    Ok("Database reset".into())
                });
            }
            KeyCode::Char('l') | KeyCode::Char('L') => {
                self.wants_logs = Some(vec![]);
            }
            KeyCode::Char('p') | KeyCode::Char('P') => {
                self.spawn_op("Pruning Docker cache", tx, |_root| async move {
                    let result = docker::docker_prune().await?;
                    Ok(format!("Pruned — {result}"))
                });
            }
            _ => {}
        }
    }

    fn handle_submenu_key(
        &mut self,
        key: KeyEvent,
        target: MenuTarget,
        tx: &mpsc::UnboundedSender<AppEvent>,
    ) {
        match key.code {
            KeyCode::Esc => {
                self.mode = Mode::Main;
            }
            KeyCode::Char('s') => {
                let label = target.label().to_string();
                let services = services::compose_names_for_target(&target);
                let svcs: Vec<String> = services.into_iter().map(String::from).collect();
                let is_infra = matches!(target, MenuTarget::Layer(Layer::Infra));

                self.spawn_op(&format!("Starting {label}"), tx, move |root| async move {
                    if svcs.is_empty() || matches!(target, MenuTarget::All) {
                        docker::compose_up(&root, &[]).await?;
                    } else {
                        let refs: Vec<&str> = svcs.iter().map(|s| s.as_str()).collect();
                        let mut all = refs.clone();
                        if is_infra {
                            all.push("minio-init");
                        }
                        docker::compose_up(&root, &all).await?;
                    }
                    Ok(format!("{label} started"))
                });
                self.mode = Mode::Main;
            }
            KeyCode::Char('x') => {
                let label = target.label().to_string();
                let services = services::compose_names_for_target(&target);
                let svcs: Vec<String> = services.into_iter().map(String::from).collect();

                self.spawn_op(&format!("Stopping {label}"), tx, move |root| async move {
                    if matches!(target, MenuTarget::All) {
                        docker::compose_down(&root).await?;
                    } else {
                        let refs: Vec<&str> = svcs.iter().map(|s| s.as_str()).collect();
                        docker::compose_stop(&root, &refs).await?;
                    }
                    Ok(format!("{label} stopped"))
                });
                self.mode = Mode::Main;
            }
            KeyCode::Char('r') => {
                let label = target.label().to_string();
                let services = services::compose_names_for_target(&target);
                let svcs: Vec<String> = services.into_iter().map(String::from).collect();

                self.spawn_op(&format!("Restarting {label}"), tx, move |root| async move {
                    if matches!(target, MenuTarget::All) {
                        docker::compose_down(&root).await?;
                        docker::compose_up(&root, &[]).await?;
                    } else {
                        let refs: Vec<&str> = svcs.iter().map(|s| s.as_str()).collect();
                        docker::compose_restart(&root, &refs).await?;
                    }
                    Ok(format!("{label} restarted"))
                });
                self.mode = Mode::Main;
            }
            KeyCode::Char('b') => {
                if target.has_rebuild()
                    && !matches!(target, MenuTarget::Layer(Layer::Infra))
                {
                    let label = target.label().to_string();
                    let services = services::compose_names_for_target(&target);
                    let svcs: Vec<String> = services.into_iter().map(String::from).collect();

                    self.spawn_op(
                        &format!("Rebuilding {label}"),
                        tx,
                        move |root| async move {
                            if matches!(target, MenuTarget::All) {
                                docker::compose_up_build(&root, &[]).await?;
                            } else {
                                let refs: Vec<&str> =
                                    svcs.iter().map(|s| s.as_str()).collect();
                                docker::compose_up_build(&root, &refs).await?;
                            }
                            Ok(format!("{label} rebuilt"))
                        },
                    );
                }
                self.mode = Mode::Main;
            }
            KeyCode::Char('l') => {
                let services = services::compose_names_for_target(&target);
                self.wants_logs =
                    Some(services.into_iter().map(String::from).collect());
                self.mode = Mode::Main;
            }
            _ => {
                // Unrecognized key — stay in submenu
            }
        }
    }

    // ── Operation spawning ──────────────────────────────────────────

    fn spawn_op<F, Fut>(
        &mut self,
        description: &str,
        tx: &mpsc::UnboundedSender<AppEvent>,
        op: F,
    ) where
        F: FnOnce(String) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = anyhow::Result<String>> + Send,
    {
        self.pending_op = Some((description.to_string(), Instant::now()));
        let tx = tx.clone();
        let root = self.repo_root.clone();

        tokio::spawn(async move {
            let result = op(root).await;
            let (success, message) = match result {
                Ok(msg) => (true, msg),
                Err(e) => (false, format!("{e}")),
            };
            let _ = tx.send(AppEvent::OpComplete { success, message });
        });
    }
}

// ── Helpers ─────────────────────────────────────────────────────────

fn open_url(url: &str) {
    #[cfg(target_os = "macos")]
    {
        let _ = std::process::Command::new("open").arg(url).spawn();
    }
    #[cfg(target_os = "linux")]
    {
        let _ = std::process::Command::new("xdg-open").arg(url).spawn();
    }
}
