//! Global state management

use dioxus::prelude::*;

/// Filter type for posts
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum PostFilter {
    #[default]
    All,
    Service,
    Opportunity,
    Business,
}

impl PostFilter {
    pub fn label(&self) -> &'static str {
        match self {
            PostFilter::All => "All Resources",
            PostFilter::Service => "Services",
            PostFilter::Opportunity => "Opportunities",
            PostFilter::Business => "Businesses",
        }
    }

    pub fn icon(&self) -> &'static str {
        match self {
            PostFilter::All => "\u{1F4CB}",       // 📋
            PostFilter::Service => "\u{1F3E5}",   // 🏥
            PostFilter::Opportunity => "\u{1F91D}", // 🤝
            PostFilter::Business => "\u{1F3EA}", // 🏪
        }
    }

    pub fn variants() -> &'static [PostFilter] {
        &[
            PostFilter::All,
            PostFilter::Service,
            PostFilter::Opportunity,
            PostFilter::Business,
        ]
    }
}

/// Chat panel state
#[derive(Clone)]
pub struct ChatState {
    pub is_open: Signal<bool>,
    pub container_id: Signal<Option<String>>,
}

impl ChatState {
    pub fn new() -> Self {
        Self {
            is_open: Signal::new(false),
            container_id: Signal::new(None),
        }
    }

    pub fn toggle(&self) {
        self.is_open.set(!self.is_open.peek());
    }

    pub fn open(&self) {
        self.is_open.set(true);
    }

    pub fn close(&self) {
        self.is_open.set(false);
    }
}
