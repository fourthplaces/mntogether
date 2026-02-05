//! Root application component

use dioxus::prelude::*;

use crate::auth::AuthProvider;
use crate::routes::Route;

/// Root application component
#[component]
pub fn App() -> Element {
    rsx! {
        // Global styles
        document::Stylesheet { href: asset!("/assets/tailwind.css") }

        // Auth context provider wraps the entire app
        AuthProvider {
            Router::<Route> {}
        }
    }
}
