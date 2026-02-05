//! Admin posts pages

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, APPROVE_POST, GET_PENDING_POSTS, REJECT_POST};
use crate::types::{GetListingsResponse, Post};

/// Admin posts list page
#[component]
pub fn AdminPosts() -> Element {
    let posts = use_server_future(fetch_pending_posts)?;
    let mut refresh_trigger = use_signal(|| 0);

    let handle_approve = move |post_id: String| {
        spawn(async move {
            if approve_post(post_id).await.is_ok() {
                refresh_trigger.set(refresh_trigger() + 1);
            }
        });
    };

    let handle_reject = move |post_id: String| {
        spawn(async move {
            if reject_post(post_id, "Rejected by admin".to_string()).await.is_ok() {
                refresh_trigger.set(refresh_trigger() + 1);
            }
        });
    };

    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Pending Posts" }

            match posts.value().as_ref() {
                Some(Ok(posts)) if !posts.is_empty() => rsx! {
                    div {
                        class: "bg-white rounded-lg shadow-sm border border-gray-200 divide-y divide-gray-200",
                        for post in posts.iter() {
                            PostRow {
                                post: post.clone(),
                                on_approve: handle_approve,
                                on_reject: handle_reject
                            }
                        }
                    }
                },
                Some(Ok(_)) => rsx! {
                    div {
                        class: "bg-white rounded-lg shadow-sm border border-gray-200 p-12 text-center",
                        p { class: "text-gray-500", "No pending posts to review." }
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        class: "bg-red-50 border border-red-200 text-red-700 p-4 rounded-lg",
                        "Error loading posts: {e}"
                    }
                },
                None => rsx! {
                    div { class: "text-center py-12", "Loading..." }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct PostRowProps {
    post: Post,
    on_approve: EventHandler<String>,
    on_reject: EventHandler<String>,
}

#[component]
fn PostRow(props: PostRowProps) -> Element {
    let post = &props.post;

    rsx! {
        div {
            class: "p-4 hover:bg-gray-50",
            div {
                class: "flex items-start justify-between",
                div {
                    class: "flex-1 min-w-0",
                    h3 { class: "text-sm font-medium text-gray-900 truncate", "{post.title}" }
                    p { class: "text-sm text-gray-500", "{post.organization_name}" }
                    if let Some(tldr) = &post.tldr {
                        p { class: "text-sm text-gray-600 mt-1 line-clamp-2", "{tldr}" }
                    }
                }
                div {
                    class: "flex items-center gap-2 ml-4",
                    button {
                        class: "px-3 py-1.5 bg-green-100 text-green-700 text-sm rounded hover:bg-green-200",
                        onclick: {
                            let id = post.id.clone();
                            move |_| props.on_approve.call(id.clone())
                        },
                        "Approve"
                    }
                    button {
                        class: "px-3 py-1.5 bg-red-100 text-red-700 text-sm rounded hover:bg-red-200",
                        onclick: {
                            let id = post.id.clone();
                            move |_| props.on_reject.call(id.clone())
                        },
                        "Reject"
                    }
                }
            }
        }
    }
}

/// Admin post detail page
#[component]
pub fn AdminPostDetail(id: String) -> Element {
    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Post Detail" }
            p { class: "text-gray-600", "Post ID: {id}" }
            // TODO: Implement full post detail view
        }
    }
}

#[server]
async fn fetch_pending_posts() -> Result<Vec<Post>, ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    struct Variables {
        first: i32,
    }

    let response: GetListingsResponse = client
        .query(GET_PENDING_POSTS, Some(Variables { first: 50 }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(response.listings.nodes)
}

#[server]
async fn approve_post(listing_id: String) -> Result<(), ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Variables {
        listing_id: String,
    }

    let _: serde_json::Value = client
        .mutate(APPROVE_POST, Some(Variables { listing_id }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
async fn reject_post(listing_id: String, reason: String) -> Result<(), ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Variables {
        listing_id: String,
        reason: String,
    }

    let _: serde_json::Value = client
        .mutate(REJECT_POST, Some(Variables { listing_id, reason }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
