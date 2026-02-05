//! Admin layout wrapper with auth protection

use dioxus::prelude::*;

use crate::auth::use_auth;
use crate::routes::Route;
use crate::state::ChatState;
use super::{AdminNav, ChatPanel, LoadingSpinner};

/// Admin layout component that provides navigation and auth protection
#[component]
pub fn AdminLayout() -> Element {
    let auth = use_auth();

    // Create chat state for the admin panel
    let chat_state = use_context_provider(ChatState::new);

    // Check authentication
    if auth.loading.read().clone() {
        return rsx! {
            div {
                class: "min-h-screen flex items-center justify-center bg-gray-100",
                LoadingSpinner {}
            }
        };
    }

    // Redirect if not authenticated or not admin
    if !auth.is_authenticated() {
        return rsx! {
            Redirect { to: Route::AdminLogin {} }
        };
    }

    if !auth.is_admin() {
        return rsx! {
            Redirect { to: Route::Home {} }
        };
    }

    rsx! {
        div {
            class: "min-h-screen bg-gray-100",

            // Navigation
            AdminNav {}

            // Main content
            main {
                class: "p-6",
                Outlet::<Route> {}
            }

            // Chat panel (floating)
            ChatPanel {
                is_open: chat_state.is_open.read().clone(),
                on_close: move |_| chat_state.close()
            }

            // Chat toggle button (FAB)
            button {
                class: "fixed bottom-6 right-6 w-14 h-14 bg-amber-500 text-white rounded-full shadow-lg hover:bg-amber-600 transition-colors flex items-center justify-center z-40",
                onclick: move |_| chat_state.toggle(),
                span { class: "text-2xl", "\u{1F4AC}" }
            }
        }
    }
}
