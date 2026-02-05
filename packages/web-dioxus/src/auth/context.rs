//! Authentication context provider

use dioxus::prelude::*;

use crate::types::AuthUser;
use super::server_fns::get_current_user;

/// Authentication context that provides user state to the entire app
#[derive(Clone)]
pub struct AuthContext {
    /// Current authenticated user (if any)
    pub user: Signal<Option<AuthUser>>,
    /// Whether auth state is still loading
    pub loading: Signal<bool>,
}

impl AuthContext {
    /// Check if the user is authenticated
    pub fn is_authenticated(&self) -> bool {
        self.user.read().is_some()
    }

    /// Check if the user is an admin
    pub fn is_admin(&self) -> bool {
        self.user.read().as_ref().map(|u| u.is_admin).unwrap_or(false)
    }

    /// Refresh the auth state from the server
    pub async fn refresh(&self) {
        match get_current_user().await {
            Ok(user) => {
                self.user.set(user);
            }
            Err(_) => {
                self.user.set(None);
            }
        }
        self.loading.set(false);
    }

    /// Clear the auth state (logout)
    pub fn clear(&self) {
        self.user.set(None);
    }
}

/// Auth provider component that wraps the app
#[component]
pub fn AuthProvider(children: Element) -> Element {
    // Create auth signals
    let user = use_signal(|| None::<AuthUser>);
    let loading = use_signal(|| true);

    // Create context
    let auth = AuthContext { user, loading };

    // Provide to children
    use_context_provider(|| auth.clone());

    // Load initial auth state
    use_effect(move || {
        let auth = auth.clone();
        spawn(async move {
            auth.refresh().await;
        });
    });

    children
}

/// Hook to access the auth context
pub fn use_auth() -> AuthContext {
    use_context::<AuthContext>()
}
