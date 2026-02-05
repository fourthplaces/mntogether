//! Admin websites pages

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, APPROVE_WEBSITE, CRAWL_WEBSITE, GET_ALL_WEBSITES};
use crate::routes::Route;
use crate::types::{GetWebsitesResponse, Website, WebsiteStatus};

/// Admin websites list page
#[component]
pub fn AdminWebsites() -> Element {
    let websites = use_server_future(fetch_websites)?;

    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Websites" }

            match websites.value().as_ref() {
                Some(Ok(websites)) if !websites.is_empty() => rsx! {
                    div {
                        class: "bg-white rounded-lg shadow-sm border border-gray-200 overflow-hidden",
                        table {
                            class: "min-w-full divide-y divide-gray-200",
                            thead {
                                class: "bg-gray-50",
                                tr {
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase", "Domain" }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase", "Status" }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase", "Listings" }
                                    th { class: "px-6 py-3 text-left text-xs font-medium text-gray-500 uppercase", "Actions" }
                                }
                            }
                            tbody {
                                class: "bg-white divide-y divide-gray-200",
                                for website in websites.iter() {
                                    WebsiteRow { website: website.clone() }
                                }
                            }
                        }
                    }
                },
                Some(Ok(_)) => rsx! {
                    div {
                        class: "bg-white rounded-lg shadow-sm border border-gray-200 p-12 text-center",
                        p { class: "text-gray-500", "No websites found." }
                    }
                },
                Some(Err(e)) => rsx! {
                    div {
                        class: "bg-red-50 border border-red-200 text-red-700 p-4 rounded-lg",
                        "Error: {e}"
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
struct WebsiteRowProps {
    website: Website,
}

#[component]
fn WebsiteRow(props: WebsiteRowProps) -> Element {
    let website = &props.website;

    let status_class = match website.status {
        WebsiteStatus::Approved => "bg-green-100 text-green-700",
        WebsiteStatus::PendingReview => "bg-yellow-100 text-yellow-700",
        WebsiteStatus::Rejected => "bg-red-100 text-red-700",
        WebsiteStatus::Suspended => "bg-gray-100 text-gray-700",
    };

    let handle_approve = {
        let id = website.id.clone();
        move |_| {
            let id = id.clone();
            spawn(async move {
                let _ = approve_website(id).await;
            });
        }
    };

    let handle_crawl = {
        let id = website.id.clone();
        move |_| {
            let id = id.clone();
            spawn(async move {
                let _ = crawl_website(id).await;
            });
        }
    };

    rsx! {
        tr {
            class: "hover:bg-gray-50",
            td {
                class: "px-6 py-4",
                Link {
                    to: Route::AdminWebsiteDetail { id: website.id.clone() },
                    class: "text-blue-600 hover:text-blue-700 font-medium",
                    "{website.domain}"
                }
            }
            td {
                class: "px-6 py-4",
                span {
                    class: "px-2 py-1 rounded text-xs font-medium {status_class}",
                    "{website.status:?}"
                }
            }
            td {
                class: "px-6 py-4 text-sm text-gray-500",
                "{website.listings_count.unwrap_or(0)}"
            }
            td {
                class: "px-6 py-4",
                div {
                    class: "flex gap-2",
                    if website.status == WebsiteStatus::PendingReview {
                        button {
                            class: "px-2 py-1 bg-green-100 text-green-700 text-xs rounded hover:bg-green-200",
                            onclick: handle_approve,
                            "Approve"
                        }
                    }
                    if website.status == WebsiteStatus::Approved {
                        button {
                            class: "px-2 py-1 bg-blue-100 text-blue-700 text-xs rounded hover:bg-blue-200",
                            onclick: handle_crawl,
                            "Crawl"
                        }
                    }
                }
            }
        }
    }
}

/// Admin website detail page
#[component]
pub fn AdminWebsiteDetail(id: String) -> Element {
    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Website Detail" }
            p { class: "text-gray-600", "Website ID: {id}" }
            // TODO: Implement full website detail view
        }
    }
}

#[server]
async fn fetch_websites() -> Result<Vec<Website>, ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    struct Variables {
        first: i32,
    }

    let response: GetWebsitesResponse = client
        .query(GET_ALL_WEBSITES, Some(Variables { first: 50 }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(response.websites.nodes)
}

#[server]
async fn approve_website(website_id: String) -> Result<(), ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Variables {
        website_id: String,
    }

    let _: serde_json::Value = client
        .mutate(APPROVE_WEBSITE, Some(Variables { website_id }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}

#[server]
async fn crawl_website(website_id: String) -> Result<(), ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Variables {
        website_id: String,
    }

    let _: serde_json::Value = client
        .mutate(CRAWL_WEBSITE, Some(Variables { website_id }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(())
}
