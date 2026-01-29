//! Core types for the interactive menu system

use crate::menu::MenuAction;

/// Lifecycle-based menu groups
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WorkflowGroup {
    /// First-time setup & recovery: Status, Doctor, Init, Sync
    Bootstrap,
    /// Day-to-day dev loop: Start/Stop, Watch, Build, Logs, Shell
    Develop,
    /// Pre-ship confidence: Tests, Coverage, Lint, Format, Check
    Validate,
    /// Deep inspection: Logs, Trace, Profile, AI Tasks
    Debug,
    /// Irreversible actions: Release, Tag, Publish
    Ship,
    /// Infrastructure & environment control: Docker, DB, Env
    Operate,
}

impl WorkflowGroup {
    /// Display label
    pub fn label(&self) -> &'static str {
        match self {
            Self::Bootstrap => "Bootstrap",
            Self::Develop => "Develop",
            Self::Validate => "Validate",
            Self::Debug => "Debug",
            Self::Ship => "Ship",
            Self::Operate => "Operate",
        }
    }

    /// Icon for the group
    pub fn icon(&self) -> &'static str {
        match self {
            Self::Bootstrap => "🚀",
            Self::Develop => "💻",
            Self::Validate => "🧪",
            Self::Debug => "🐞",
            Self::Ship => "📦",
            Self::Operate => "⚙️",
        }
    }

    /// All groups in display order
    pub fn all() -> &'static [WorkflowGroup] {
        &[
            Self::Bootstrap,
            Self::Develop,
            Self::Validate,
            Self::Debug,
            Self::Ship,
            Self::Operate,
        ]
    }
}

/// Live status indicator for menu items
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum ItemStatus {
    #[default]
    None,
    /// Service/container is running
    Running,
    /// Service/container is stopped
    Stopped,
    /// Has pending items (migrations, etc.)
    Pending(u32),
    /// Success state (CI passed, tests green)
    Success,
    /// Failure state (CI failed)
    Failure,
    /// Needs attention
    Warning,
}

impl ItemStatus {
    /// Status indicator character
    pub fn indicator(&self) -> &'static str {
        match self {
            Self::None => " ",
            Self::Running => "●",
            Self::Stopped => "○",
            Self::Pending(_) => "◐",
            Self::Success => "✓",
            Self::Failure => "✗",
            Self::Warning => "!",
        }
    }

    /// ANSI color for the indicator
    pub fn color(&self) -> console::Color {
        match self {
            Self::None => console::Color::White,
            Self::Running => console::Color::Green,
            Self::Stopped => console::Color::Red,
            Self::Pending(_) => console::Color::Yellow,
            Self::Success => console::Color::Green,
            Self::Failure => console::Color::Red,
            Self::Warning => console::Color::Yellow,
        }
    }
}

/// An interactive menu item with metadata for search, status, and shortcuts
#[derive(Debug, Clone)]
pub struct InteractiveMenuItem {
    /// Unique identifier (e.g., "docker:up", "dev:start")
    pub id: String,
    /// Display label
    pub label: String,
    /// The action to execute
    pub action: MenuAction,
    /// Which workflow group this belongs to
    pub group: WorkflowGroup,
    /// Single-key shortcut (optional)
    pub quick_key: Option<char>,
    /// Live status indicator
    pub status: ItemStatus,
    /// Whether user has pinned this item
    pub is_pinned: bool,
    /// Keywords for fuzzy search (in addition to label)
    pub keywords: Vec<String>,
}

impl InteractiveMenuItem {
    /// Create a new menu item
    pub fn new(id: &str, label: &str, action: MenuAction, group: WorkflowGroup) -> Self {
        Self {
            id: id.to_string(),
            label: label.to_string(),
            action,
            group,
            quick_key: None,
            status: ItemStatus::None,
            is_pinned: false,
            keywords: Vec::new(),
        }
    }

    /// Add a quick key shortcut
    pub fn with_key(mut self, key: char) -> Self {
        self.quick_key = Some(key);
        self
    }

    /// Add search keywords
    pub fn with_keywords(mut self, keywords: &[&str]) -> Self {
        self.keywords = keywords.iter().map(|s| s.to_string()).collect();
        self
    }

    /// Set the status
    pub fn with_status(mut self, status: ItemStatus) -> Self {
        self.status = status;
        self
    }

    /// Mark as pinned
    pub fn pinned(mut self) -> Self {
        self.is_pinned = true;
        self
    }

    /// Get all searchable text (label + keywords)
    pub fn searchable_text(&self) -> String {
        let mut text = self.label.clone();
        for kw in &self.keywords {
            text.push(' ');
            text.push_str(kw);
        }
        text
    }
}

/// A group of menu items that can be expanded/collapsed
#[derive(Debug, Clone)]
pub struct MenuGroup {
    pub group: WorkflowGroup,
    pub items: Vec<InteractiveMenuItem>,
    pub expanded: bool,
}

impl MenuGroup {
    pub fn new(group: WorkflowGroup, items: Vec<InteractiveMenuItem>) -> Self {
        Self {
            group,
            items,
            expanded: false,
        }
    }
}

/// Full interactive menu state
#[derive(Debug, Default)]
pub struct InteractiveMenuState {
    /// All workflow groups with their items
    pub groups: Vec<MenuGroup>,
    /// User's pinned items (shown at top)
    pub pinned: Vec<InteractiveMenuItem>,
    /// Current search query (None = not searching)
    pub search_query: Option<String>,
    /// Filtered items when searching
    pub filtered_items: Vec<InteractiveMenuItem>,
    /// Currently selected index
    pub selected_index: usize,
}

impl InteractiveMenuState {
    /// Get all items in display order (pinned, then groups)
    pub fn all_items(&self) -> Vec<&InteractiveMenuItem> {
        let mut items = Vec::new();

        // Pinned items first
        for item in &self.pinned {
            items.push(item);
        }

        // Group items (only expanded groups)
        for group in &self.groups {
            if group.expanded {
                for item in &group.items {
                    items.push(item);
                }
            }
        }

        items
    }

    /// Check if we're in search mode
    pub fn is_searching(&self) -> bool {
        self.search_query.is_some()
    }
}
