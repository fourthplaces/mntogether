//! Admin login page

use dioxus::prelude::*;

use crate::auth::{send_verification_code, use_auth, verify_code};
use crate::routes::Route;

#[derive(Clone, Copy, PartialEq)]
enum LoginStep {
    Identifier,
    Code,
}

/// Admin login page
#[component]
pub fn AdminLogin() -> Element {
    let auth = use_auth();
    let navigator = use_navigator();

    let mut identifier = use_signal(String::new);
    let mut code = use_signal(String::new);
    let mut step = use_signal(|| LoginStep::Identifier);
    let mut error = use_signal(|| None::<String>);
    let mut is_pending = use_signal(|| false);

    // Redirect if already authenticated
    if auth.is_authenticated() && auth.is_admin() {
        return rsx! {
            Redirect { to: Route::AdminDashboard {} }
        };
    }

    let handle_send_code = move |_| {
        let id = identifier().trim().to_string();
        if id.is_empty() {
            error.set(Some("Please enter your phone number or email".to_string()));
            return;
        }

        spawn(async move {
            is_pending.set(true);
            error.set(None);

            match send_verification_code(id).await {
                Ok(true) => step.set(LoginStep::Code),
                Ok(false) => error.set(Some("Failed to send verification code".to_string())),
                Err(e) => error.set(Some(e.to_string())),
            }

            is_pending.set(false);
        });
    };

    let handle_verify = move |_| {
        let id = identifier().trim().to_string();
        let c = code().trim().to_string();

        if c.is_empty() {
            error.set(Some("Please enter the verification code".to_string()));
            return;
        }

        spawn(async move {
            is_pending.set(true);
            error.set(None);

            match verify_code(id, c).await {
                Ok(Some(_token)) => {
                    // Refresh auth state and redirect
                    auth.refresh().await;
                    navigator.push(Route::AdminDashboard {});
                }
                Ok(None) => error.set(Some("Invalid verification code".to_string())),
                Err(e) => error.set(Some(e.to_string())),
            }

            is_pending.set(false);
        });
    };

    rsx! {
        div {
            class: "min-h-screen bg-gray-100 flex items-center justify-center px-4",

            div {
                class: "bg-white rounded-lg shadow-md p-8 max-w-md w-full",

                div {
                    class: "mb-6 text-center",
                    h1 { class: "text-2xl font-bold text-gray-900 mb-2", "Admin Login" }
                    p { class: "text-gray-600 text-sm", "MN Together" }
                }

                if let Some(err) = error() {
                    div {
                        class: "mb-4 p-3 bg-orange-50 border border-orange-200 text-orange-800 rounded text-sm",
                        "{err}"
                    }
                }

                match step() {
                    LoginStep::Identifier => rsx! {
                        form {
                            onsubmit: handle_send_code,
                            div {
                                class: "mb-4",
                                label {
                                    class: "block text-sm font-medium text-gray-700 mb-2",
                                    "Phone Number or Email"
                                }
                                input {
                                    r#type: "text",
                                    value: "{identifier}",
                                    oninput: move |e| identifier.set(e.value()),
                                    placeholder: "+1234567890 or admin@example.com",
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500",
                                    disabled: is_pending()
                                }
                                p {
                                    class: "mt-1 text-xs text-gray-500",
                                    "Enter your registered phone number (with country code) or email address"
                                }
                            }
                            button {
                                r#type: "submit",
                                class: "w-full bg-amber-700 text-white py-2 px-4 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed",
                                disabled: is_pending(),
                                if is_pending() { "Sending..." } else { "Send Verification Code" }
                            }
                        }
                    },
                    LoginStep::Code => rsx! {
                        form {
                            onsubmit: handle_verify,
                            div {
                                class: "mb-4",
                                label {
                                    class: "block text-sm font-medium text-gray-700 mb-2",
                                    "Verification Code"
                                }
                                input {
                                    r#type: "text",
                                    value: "{code}",
                                    oninput: move |e| code.set(e.value()),
                                    placeholder: "Enter 6-digit code",
                                    class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500",
                                    disabled: is_pending()
                                }
                                p {
                                    class: "mt-1 text-xs text-gray-500",
                                    "Enter the verification code sent to {identifier}"
                                }
                            }
                            div {
                                class: "space-y-2",
                                button {
                                    r#type: "submit",
                                    class: "w-full bg-amber-700 text-white py-2 px-4 rounded-md hover:bg-amber-800 focus:outline-none focus:ring-2 focus:ring-amber-500 focus:ring-offset-2 disabled:opacity-50 disabled:cursor-not-allowed",
                                    disabled: is_pending(),
                                    if is_pending() { "Verifying..." } else { "Verify & Sign In" }
                                }
                                button {
                                    r#type: "button",
                                    class: "w-full bg-stone-100 text-stone-700 py-2 px-4 rounded-md hover:bg-stone-200 focus:outline-none focus:ring-2 focus:ring-stone-500 focus:ring-offset-2",
                                    onclick: move |_| {
                                        step.set(LoginStep::Identifier);
                                        code.set(String::new());
                                        error.set(None);
                                    },
                                    "Back"
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}
