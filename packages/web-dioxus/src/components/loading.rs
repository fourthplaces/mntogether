//! Loading components

use dioxus::prelude::*;

/// Full-page loading spinner
#[component]
pub fn LoadingSpinner() -> Element {
    rsx! {
        div {
            class: "flex flex-col items-center justify-center",
            div {
                class: "flex space-x-2",
                div { class: "w-3 h-3 bg-amber-400 rounded-full animate-bounce" }
                div { class: "w-3 h-3 bg-amber-400 rounded-full animate-bounce", style: "animation-delay: 0.1s" }
                div { class: "w-3 h-3 bg-amber-400 rounded-full animate-bounce", style: "animation-delay: 0.2s" }
            }
            p { class: "mt-4 text-sm text-gray-500", "Loading..." }
        }
    }
}

/// Inline loading indicator
#[component]
pub fn LoadingDots() -> Element {
    rsx! {
        div {
            class: "inline-flex space-x-1",
            div { class: "w-2 h-2 bg-gray-400 rounded-full animate-bounce" }
            div { class: "w-2 h-2 bg-gray-400 rounded-full animate-bounce", style: "animation-delay: 0.1s" }
            div { class: "w-2 h-2 bg-gray-400 rounded-full animate-bounce", style: "animation-delay: 0.2s" }
        }
    }
}
