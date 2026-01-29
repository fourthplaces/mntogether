//! User preferences for pinned items and custom quick keys

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

use super::types::WorkflowGroup;

/// User preferences stored in ~/.config/mntogether-dev/prefs.json
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct UserPreferences {
    /// Pinned action IDs (shown at top of menu)
    #[serde(default)]
    pub pins: Vec<String>,

    /// Custom quick key overrides (action_id -> key)
    #[serde(default)]
    pub quick_keys: HashMap<String, char>,

    /// Collapsed groups (user can collapse groups they don't use)
    #[serde(default)]
    pub collapsed_groups: Vec<String>,
}

impl UserPreferences {
    /// Load preferences from disk
    pub fn load() -> Self {
        if let Some(path) = prefs_path() {
            if let Ok(data) = fs::read_to_string(&path) {
                if let Ok(prefs) = serde_json::from_str(&data) {
                    return prefs;
                }
            }
        }
        Self::default()
    }

    /// Save preferences to disk
    pub fn save(&self) {
        if let Some(path) = prefs_path() {
            if let Some(parent) = path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Ok(data) = serde_json::to_string_pretty(self) {
                let _ = fs::write(&path, data);
            }
        }
    }

    /// Check if an action is pinned
    pub fn is_pinned(&self, action_id: &str) -> bool {
        self.pins.contains(&action_id.to_string())
    }

    /// Pin an action
    pub fn pin(&mut self, action_id: &str) {
        if !self.is_pinned(action_id) {
            self.pins.push(action_id.to_string());
            self.save();
        }
    }

    /// Unpin an action
    pub fn unpin(&mut self, action_id: &str) {
        self.pins.retain(|id| id != action_id);
        self.save();
    }

    /// Toggle pin state
    pub fn toggle_pin(&mut self, action_id: &str) {
        if self.is_pinned(action_id) {
            self.unpin(action_id);
        } else {
            self.pin(action_id);
        }
    }

    /// Get custom quick key for an action (if set)
    pub fn get_quick_key(&self, action_id: &str) -> Option<char> {
        self.quick_keys.get(action_id).copied()
    }

    /// Set a custom quick key
    pub fn set_quick_key(&mut self, action_id: &str, key: char) {
        self.quick_keys.insert(action_id.to_string(), key);
        self.save();
    }

    /// Check if a group is collapsed
    pub fn is_collapsed(&self, group: WorkflowGroup) -> bool {
        self.collapsed_groups.contains(&group.label().to_string())
    }

    /// Toggle group collapse state
    pub fn toggle_collapse(&mut self, group: WorkflowGroup) {
        let label = group.label().to_string();
        if self.collapsed_groups.contains(&label) {
            self.collapsed_groups.retain(|g| g != &label);
        } else {
            self.collapsed_groups.push(label);
        }
        self.save();
    }
}

/// Get the path to the preferences file
fn prefs_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("mntogether-dev").join("prefs.json"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pin_unpin() {
        let mut prefs = UserPreferences::default();
        assert!(!prefs.is_pinned("docker:up"));

        prefs.pin("docker:up");
        assert!(prefs.is_pinned("docker:up"));

        prefs.unpin("docker:up");
        assert!(!prefs.is_pinned("docker:up"));
    }

    #[test]
    fn test_toggle_pin() {
        let mut prefs = UserPreferences::default();
        prefs.toggle_pin("dev:start");
        assert!(prefs.is_pinned("dev:start"));
        prefs.toggle_pin("dev:start");
        assert!(!prefs.is_pinned("dev:start"));
    }
}
