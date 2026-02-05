//! Submit resource page component

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, SUBMIT_RESOURCE_LINK};
use crate::routes::Route;
use crate::types::SubmitResourceLinkResponse;

/// Submit page - submit a new resource link
#[component]
pub fn Submit() -> Element {
    let mut url = use_signal(String::new);
    let mut context = use_signal(String::new);
    let mut contact = use_signal(String::new);
    let mut is_submitting = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);
    let mut success = use_signal(|| false);

    let is_valid_url = use_memo(move || {
        let u = url();
        u.starts_with("http://") || u.starts_with("https://")
    });

    let handle_submit = move |_| {
        if !is_valid_url() || is_submitting() {
            return;
        }

        let url_value = url().trim().to_string();
        let context_value = context().trim().to_string();
        let contact_value = contact().trim().to_string();

        spawn(async move {
            is_submitting.set(true);
            error.set(None);

            match submit_resource(url_value, context_value, contact_value).await {
                Ok(_) => {
                    success.set(true);
                    url.set(String::new());
                    context.set(String::new());
                    contact.set(String::new());
                }
                Err(e) => {
                    error.set(Some(e.to_string()));
                }
            }

            is_submitting.set(false);
        });
    };

    rsx! {
        div {
            class: "min-h-screen bg-gradient-to-b from-blue-50 to-white",

            // Header
            header {
                class: "bg-white border-b border-gray-100",
                div {
                    class: "max-w-2xl mx-auto px-4 py-8",
                    Link {
                        to: Route::Home {},
                        class: "text-blue-600 hover:text-blue-700 text-sm mb-4 inline-block",
                        "\u{2190} Back to Home"
                    }
                    h1 {
                        class: "text-3xl font-bold text-gray-900 mb-2",
                        "Submit a Resource"
                    }
                    p {
                        class: "text-gray-600",
                        "Share a helpful resource, service, or organization with the community."
                    }
                }
            }

            // Form
            main {
                class: "max-w-2xl mx-auto px-4 py-8",

                if success() {
                    div {
                        class: "bg-green-50 border border-green-200 text-green-700 p-6 rounded-lg text-center",
                        h3 { class: "text-lg font-semibold mb-2", "Thank you!" }
                        p { class: "mb-4", "Your resource has been submitted and is being processed." }
                        button {
                            class: "px-4 py-2 bg-green-600 text-white rounded-lg hover:bg-green-700 transition-colors",
                            onclick: move |_| success.set(false),
                            "Submit Another"
                        }
                    }
                } else {
                    form {
                        class: "bg-white rounded-lg shadow-sm border border-gray-200 p-6 space-y-6",
                        onsubmit: handle_submit,

                        if let Some(err) = error() {
                            div {
                                class: "bg-red-50 border border-red-200 text-red-700 p-4 rounded-lg",
                                "{err}"
                            }
                        }

                        // URL field
                        div {
                            label {
                                class: "block text-sm font-medium text-gray-700 mb-2",
                                "Resource URL "
                                span { class: "text-red-500", "*" }
                            }
                            input {
                                r#type: "url",
                                value: "{url}",
                                oninput: move |e| url.set(e.value()),
                                placeholder: "https://example.org/services",
                                class: "w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500",
                                required: true
                            }
                            p {
                                class: "mt-1 text-xs text-gray-500",
                                "The URL of the resource, service, or organization page"
                            }
                        }

                        // Context field
                        div {
                            label {
                                class: "block text-sm font-medium text-gray-700 mb-2",
                                "Additional Context"
                            }
                            textarea {
                                value: "{context}",
                                oninput: move |e| context.set(e.value()),
                                placeholder: "Tell us about this resource and why it's helpful...",
                                rows: "4",
                                class: "w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500 resize-none"
                            }
                        }

                        // Contact field
                        div {
                            label {
                                class: "block text-sm font-medium text-gray-700 mb-2",
                                "Your Contact (optional)"
                            }
                            input {
                                r#type: "text",
                                value: "{contact}",
                                oninput: move |e| contact.set(e.value()),
                                placeholder: "Email or phone (for follow-up questions)",
                                class: "w-full px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                            }
                            p {
                                class: "mt-1 text-xs text-gray-500",
                                "Optional - we may reach out if we have questions"
                            }
                        }

                        // Submit button
                        button {
                            r#type: "submit",
                            class: "w-full py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: !is_valid_url() || is_submitting(),
                            if is_submitting() {
                                "Submitting..."
                            } else {
                                "Submit Resource"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[server]
async fn submit_resource(
    url: String,
    context: String,
    contact: String,
) -> Result<SubmitResourceLinkResponse, ServerFnError> {
    let api_url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(api_url);

    #[derive(serde::Serialize)]
    struct Input {
        url: String,
        context: Option<String>,
        submitter_contact: Option<String>,
    }

    #[derive(serde::Serialize)]
    struct Variables {
        input: Input,
    }

    client
        .mutate(
            SUBMIT_RESOURCE_LINK,
            Some(Variables {
                input: Input {
                    url,
                    context: if context.is_empty() { None } else { Some(context) },
                    submitter_contact: if contact.is_empty() { None } else { Some(contact) },
                },
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}
