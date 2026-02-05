//! Admin resources pages

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, APPROVE_RESOURCE, GET_RESOURCES};
use crate::routes::Route;
use crate::types::{Resource, ResourceStatus};

/// Admin resources list page
#[component]
pub fn AdminResources() -> Element {
    let resources = use_server_future(fetch_resources)?;

    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Resources" }

            match resources.value().as_ref() {
                Some(Ok(resources)) if !resources.is_empty() => rsx! {
                    div {
                        class: "bg-white rounded-lg shadow-sm border border-gray-200 divide-y divide-gray-200",
                        for resource in resources.iter() {
                            ResourceRow { resource: resource.clone() }
                        }
                    }
                },
                Some(Ok(_)) => rsx! {
                    div {
                        class: "bg-white rounded-lg shadow-sm border border-gray-200 p-12 text-center",
                        p { class: "text-gray-500", "No pending resources." }
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
struct ResourceRowProps {
    resource: Resource,
}

#[component]
fn ResourceRow(props: ResourceRowProps) -> Element {
    let resource = &props.resource;

    let status_class = match resource.status {
        ResourceStatus::Approved => "bg-green-100 text-green-700",
        ResourceStatus::Pending => "bg-yellow-100 text-yellow-700",
        ResourceStatus::Rejected => "bg-red-100 text-red-700",
    };

    rsx! {
        div {
            class: "p-4 hover:bg-gray-50",
            div {
                class: "flex items-start justify-between",
                div {
                    class: "flex-1 min-w-0",
                    Link {
                        to: Route::AdminResourceDetail { id: resource.id.clone() },
                        class: "text-sm font-medium text-blue-600 hover:text-blue-700",
                        "{resource.title}"
                    }
                    if let Some(org) = &resource.organization_name {
                        p { class: "text-sm text-gray-500", "{org}" }
                    }
                    p { class: "text-sm text-gray-600 mt-1 line-clamp-2", "{resource.content}" }
                }
                div {
                    class: "ml-4",
                    span {
                        class: "px-2 py-1 rounded text-xs font-medium {status_class}",
                        "{resource.status:?}"
                    }
                }
            }
        }
    }
}

/// Admin resource detail page
#[component]
pub fn AdminResourceDetail(id: String) -> Element {
    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Resource Detail" }
            p { class: "text-gray-600", "Resource ID: {id}" }
            // TODO: Implement full resource detail view
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetResourcesResponse {
    resources: crate::types::PaginatedResult<Resource>,
}

#[server]
async fn fetch_resources() -> Result<Vec<Resource>, ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    struct Variables {
        first: i32,
        status: String,
    }

    let response: GetResourcesResponse = client
        .query(
            GET_RESOURCES,
            Some(Variables {
                first: 50,
                status: "PENDING".to_string(),
            }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(response.resources.nodes)
}
