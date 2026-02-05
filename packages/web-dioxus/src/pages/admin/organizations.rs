//! Admin organizations pages

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, GET_ORGANIZATIONS};
use crate::routes::Route;
use crate::types::{GetOrganizationsResponse, Organization};

/// Admin organizations list page
#[component]
pub fn AdminOrganizations() -> Element {
    let orgs = use_server_future(fetch_organizations)?;

    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Organizations" }

            match orgs.value().as_ref() {
                Some(Ok(orgs)) if !orgs.is_empty() => rsx! {
                    div {
                        class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4",
                        for org in orgs.iter() {
                            OrgCard { org: org.clone() }
                        }
                    }
                },
                Some(Ok(_)) => rsx! {
                    div {
                        class: "bg-white rounded-lg shadow-sm border border-gray-200 p-12 text-center",
                        p { class: "text-gray-500", "No organizations found." }
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
struct OrgCardProps {
    org: Organization,
}

#[component]
fn OrgCard(props: OrgCardProps) -> Element {
    let org = &props.org;

    rsx! {
        Link {
            to: Route::AdminOrganizationDetail { id: org.id.clone() },
            class: "block bg-white rounded-lg shadow-sm border border-gray-200 p-4 hover:shadow-md transition-shadow",
            h3 { class: "font-medium text-gray-900 mb-1", "{org.name}" }
            if let Some(description) = &org.description {
                p { class: "text-sm text-gray-600 line-clamp-2", "{description}" }
            }
            if let Some(location) = &org.location {
                p { class: "text-xs text-gray-500 mt-2", "\u{1F4CD} {location}" }
            }
        }
    }
}

/// Admin organization detail page
#[component]
pub fn AdminOrganizationDetail(id: String) -> Element {
    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Organization Detail" }
            p { class: "text-gray-600", "Organization ID: {id}" }
            // TODO: Implement full organization detail view
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct GetOrganizationsResponse {
    organizations: crate::types::PaginatedResult<Organization>,
}

#[server]
async fn fetch_organizations() -> Result<Vec<Organization>, ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    struct Variables {
        first: i32,
    }

    let response: GetOrganizationsResponse = client
        .query(GET_ORGANIZATIONS, Some(Variables { first: 50 }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(response.organizations.nodes)
}
