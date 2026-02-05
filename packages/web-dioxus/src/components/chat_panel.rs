//! Chat panel component for admin assistant

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, CREATE_CHAT, GET_MESSAGES, GET_RECENT_CHATS, SEND_MESSAGE};
use crate::types::{ChatMessage, CreateChatResponse, GetMessagesResponse, GetRecentChatsResponse, SendMessageResponse};

#[derive(Props, Clone, PartialEq)]
pub struct ChatPanelProps {
    pub is_open: bool,
    pub on_close: EventHandler<()>,
}

/// Chat panel component
#[component]
pub fn ChatPanel(props: ChatPanelProps) -> Element {
    let mut container_id = use_signal(|| None::<String>);
    let mut messages = use_signal(Vec::<ChatMessage>::new);
    let mut input = use_signal(String::new);
    let mut is_typing = use_signal(|| false);
    let mut is_sending = use_signal(|| false);
    let mut is_loading = use_signal(|| true);

    // Load initial chat on open
    use_effect(move || {
        if props.is_open && container_id.read().is_none() {
            spawn(async move {
                is_loading.set(true);

                // Try to restore recent chat
                if let Ok(response) = fetch_recent_chats().await {
                    if let Some(chat) = response.recent_chats.first() {
                        container_id.set(Some(chat.id.clone()));
                        // Load messages
                        if let Ok(msg_response) = fetch_messages(&chat.id).await {
                            messages.set(msg_response.messages);
                        }
                    } else {
                        // Create new chat
                        if let Ok(response) = create_chat().await {
                            container_id.set(Some(response.create_chat.id));
                        }
                    }
                }

                is_loading.set(false);
            });
        }
    });

    // Poll for new messages
    use_effect(move || {
        let id = container_id.read().clone();
        if props.is_open && id.is_some() {
            spawn(async move {
                loop {
                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                    if let Some(id) = container_id.read().as_ref() {
                        if let Ok(response) = fetch_messages(id).await {
                            messages.set(response.messages);
                        }
                    }
                }
            });
        }
    });

    let send_message = move |_| {
        let content = input.read().trim().to_string();
        if content.is_empty() || is_sending.read().clone() {
            return;
        }

        let id = container_id.read().clone();
        if let Some(id) = id {
            input.set(String::new());
            is_typing.set(true);
            is_sending.set(true);

            spawn(async move {
                if let Ok(_) = send_chat_message(&id, &content).await {
                    // Refetch messages
                    if let Ok(response) = fetch_messages(&id).await {
                        messages.set(response.messages);
                    }
                }
                is_typing.set(false);
                is_sending.set(false);
            });
        }
    };

    let start_new_chat = move |_| {
        spawn(async move {
            is_loading.set(true);
            if let Ok(response) = create_chat().await {
                container_id.set(Some(response.create_chat.id));
                messages.set(vec![]);
            }
            is_loading.set(false);
        });
    };

    if !props.is_open {
        return None;
    }

    rsx! {
        div {
            class: "fixed inset-y-0 right-0 w-96 bg-white shadow-xl border-l border-stone-200 flex flex-col z-50",

            // Header
            div {
                class: "flex items-center justify-between px-4 py-3 border-b border-stone-200 bg-amber-50",
                div {
                    class: "flex items-center gap-2",
                    span { class: "text-xl", "\u{1F4AC}" }
                    h2 { class: "font-semibold text-stone-900", "Admin Assistant" }
                }
                div {
                    class: "flex items-center gap-2",
                    button {
                        class: "text-stone-500 hover:text-stone-700 text-sm px-2 py-1 rounded hover:bg-stone-100",
                        onclick: start_new_chat,
                        "+ New"
                    }
                    button {
                        class: "text-stone-500 hover:text-stone-700 p-1 rounded hover:bg-stone-100",
                        onclick: move |_| props.on_close.call(()),
                        // X icon
                        svg {
                            class: "w-5 h-5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M6 18L18 6M6 6l12 12"
                            }
                        }
                    }
                }
            }

            // Messages
            div {
                class: "flex-1 overflow-y-auto p-4 space-y-4",
                if is_loading.read().clone() {
                    div {
                        class: "flex flex-col items-center justify-center h-full text-center",
                        div {
                            class: "flex space-x-2 mb-4",
                            div { class: "w-3 h-3 bg-amber-400 rounded-full animate-bounce" }
                            div { class: "w-3 h-3 bg-amber-400 rounded-full animate-bounce", style: "animation-delay: 0.1s" }
                            div { class: "w-3 h-3 bg-amber-400 rounded-full animate-bounce", style: "animation-delay: 0.2s" }
                        }
                        p { class: "text-sm text-stone-500", "Starting assistant..." }
                    }
                } else {
                    for message in messages.read().iter() {
                        MessageBubble { message: message.clone() }
                    }
                    if is_typing.read().clone() {
                        div {
                            class: "flex justify-start",
                            div {
                                class: "bg-stone-100 text-stone-900 rounded-lg px-4 py-2",
                                div {
                                    class: "flex space-x-1",
                                    div { class: "w-2 h-2 bg-stone-400 rounded-full animate-bounce" }
                                    div { class: "w-2 h-2 bg-stone-400 rounded-full animate-bounce", style: "animation-delay: 0.1s" }
                                    div { class: "w-2 h-2 bg-stone-400 rounded-full animate-bounce", style: "animation-delay: 0.2s" }
                                }
                            }
                        }
                    }
                }
            }

            // Input
            if container_id.read().is_some() {
                form {
                    class: "border-t border-stone-200 p-4",
                    onsubmit: send_message,
                    div {
                        class: "flex gap-2",
                        input {
                            r#type: "text",
                            value: "{input}",
                            oninput: move |e| input.set(e.value()),
                            placeholder: "Type a message...",
                            class: "flex-1 px-4 py-2 border border-stone-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-amber-500 focus:border-transparent",
                            disabled: is_sending.read().clone()
                        }
                        button {
                            r#type: "submit",
                            class: "px-4 py-2 bg-amber-500 text-white rounded-lg hover:bg-amber-600 transition-colors disabled:opacity-50 disabled:cursor-not-allowed",
                            disabled: input.read().trim().is_empty() || is_sending.read().clone(),
                            // Send icon
                            svg {
                                class: "w-5 h-5",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M12 19l9 2-9-18-9 18 9-2zm0 0v-8"
                                }
                            }
                        }
                    }
                }

                // Quick actions
                div {
                    class: "border-t border-stone-200 p-3 bg-stone-50",
                    p { class: "text-xs text-stone-500 mb-2", "Quick actions:" }
                    div {
                        class: "flex flex-wrap gap-1",
                        for action in ["Show pending websites", "Scrape a URL", "Run agent search", "System status"] {
                            button {
                                class: "text-xs px-2 py-1 bg-white border border-stone-200 rounded-full text-stone-600 hover:bg-stone-100 hover:border-stone-300",
                                onclick: move |_| input.set(action.to_string()),
                                "{action}"
                            }
                        }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct MessageBubbleProps {
    message: ChatMessage,
}

#[component]
fn MessageBubble(props: MessageBubbleProps) -> Element {
    let is_user = props.message.role == "user";

    rsx! {
        div {
            class: if is_user { "flex justify-end" } else { "flex justify-start" },
            div {
                class: if is_user {
                    "max-w-[80%] rounded-lg px-4 py-2 bg-amber-500 text-white"
                } else {
                    "max-w-[80%] rounded-lg px-4 py-2 bg-stone-100 text-stone-900"
                },
                p { class: "text-sm whitespace-pre-wrap", "{props.message.content}" }
                p {
                    class: if is_user { "text-xs mt-1 text-amber-200" } else { "text-xs mt-1 text-stone-400" },
                    "{format_time(&props.message.created_at)}"
                }
            }
        }
    }
}

fn format_time(date_str: &str) -> String {
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(date_str) {
        date.format("%H:%M").to_string()
    } else {
        String::new()
    }
}

// Server functions for chat
#[server]
async fn fetch_recent_chats() -> Result<GetRecentChatsResponse, ServerFnError> {
    let client = server_client();

    #[derive(serde::Serialize)]
    struct Vars {
        limit: i32,
    }

    client
        .query(GET_RECENT_CHATS, Some(Vars { limit: 1 }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
async fn fetch_messages(container_id: &str) -> Result<GetMessagesResponse, ServerFnError> {
    let client = server_client();

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Vars {
        container_id: String,
    }

    client
        .query(
            GET_MESSAGES,
            Some(Vars {
                container_id: container_id.to_string(),
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
async fn create_chat() -> Result<CreateChatResponse, ServerFnError> {
    let client = server_client();

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Vars {
        language: String,
        with_agent: String,
    }

    client
        .mutate(
            CREATE_CHAT,
            Some(Vars {
                language: "en".to_string(),
                with_agent: "admin".to_string(),
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[server]
async fn send_chat_message(container_id: &str, content: &str) -> Result<SendMessageResponse, ServerFnError> {
    let client = server_client();

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Vars {
        container_id: String,
        content: String,
    }

    client
        .mutate(
            SEND_MESSAGE,
            Some(Vars {
                container_id: container_id.to_string(),
                content: content.to_string(),
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))
}

#[cfg(feature = "server")]
fn server_client() -> GraphQLClient {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    GraphQLClient::new(url)
}
