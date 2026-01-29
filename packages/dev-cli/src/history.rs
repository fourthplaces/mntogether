//! Recent actions history management

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

const MAX_RECENT_ACTIONS: usize = 5;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RecentAction {
    action: String,
    timestamp: u64,
}

#[derive(Debug, Default, Serialize, Deserialize)]
struct History {
    recent_actions: Vec<RecentAction>,
}

fn history_path() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("mntogether-dev").join("history.json"))
}

fn load_history() -> History {
    if let Some(path) = history_path() {
        if let Ok(data) = fs::read_to_string(&path) {
            if let Ok(history) = serde_json::from_str(&data) {
                return history;
            }
        }
    }
    History::default()
}

fn save_history(history: &History) {
    if let Some(path) = history_path() {
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        if let Ok(data) = serde_json::to_string_pretty(history) {
            let _ = fs::write(&path, data);
        }
    }
}

/// Record an action in the history
pub fn record_action(action: &str) {
    let mut history = load_history();
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    // Remove existing entry for this action if present
    history.recent_actions.retain(|a| a.action != action);

    // Add to front
    history.recent_actions.insert(
        0,
        RecentAction {
            action: action.to_string(),
            timestamp,
        },
    );

    // Trim to max size
    history.recent_actions.truncate(MAX_RECENT_ACTIONS);

    save_history(&history);
}

/// Get recent actions (most recent first)
pub fn get_recent_actions() -> Vec<String> {
    load_history()
        .recent_actions
        .into_iter()
        .map(|a| a.action)
        .collect()
}
