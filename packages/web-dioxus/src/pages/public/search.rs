//! Search page component

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, SEARCH_ORGANIZATIONS};
use crate::routes::Route;
use crate::types::{OrganizationMatch, SearchOrganizationsResponse};

/// Search page - semantic search for organizations
#[component]
pub fn Search() -> Element {
    let mut query = use_signal(String::new);
    let mut results = use_signal(Vec::<OrganizationMatch>::new);
    let mut is_searching = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    // Check for query param on load
    use_effect(move || {
        #[cfg(feature = "web")]
        {
            if let Some(window) = web_sys::window() {
                if let Ok(search) = window.location().search() {
                    if let Some(q) = search.strip_prefix("?q=") {
                        let decoded = urlencoding::decode(q).unwrap_or_default().to_string();
                        if !decoded.is_empty() {
                            query.set(decoded.clone());
                            spawn(async move {
                                do_search(decoded, &mut results, &mut is_searching, &mut error).await;
                            });
                        }
                    }
                }
            }
        }
    });

    let handle_search = move |_| {
        let q = query().trim().to_string();
        if q.is_empty() {
            return;
        }

        spawn(async move {
            do_search(q, &mut results, &mut is_searching, &mut error).await;
        });
    };

    rsx! {
        div {
            class: "min-h-screen bg-gradient-to-b from-blue-50 to-white",

            // Header
            header {
                class: "bg-white border-b border-gray-100",
                div {
                    class: "max-w-4xl mx-auto px-4 py-8",
                    Link {
                        to: Route::Home {},
                        class: "text-blue-600 hover:text-blue-700 text-sm mb-4 inline-block",
                        "\u{2190} Back to Home"
                    }
                    h1 {
                        class: "text-3xl font-bold text-gray-900 mb-2",
                        "Search Organizations"
                    }
                    p {
                        class: "text-gray-600",
                        "Find organizations using semantic search"
                    }
                }
            }

            // Search Form
            div {
                class: "max-w-4xl mx-auto px-4 py-6",
                form {
                    class: "flex gap-3",
                    onsubmit: handle_search,
                    input {
                        r#type: "text",
                        value: "{query}",
                        oninput: move |e| query.set(e.value()),
                        placeholder: "e.g., food assistance, job training, mental health...",
                        class: "flex-1 px-4 py-3 border border-gray-300 rounded-lg focus:outline-none focus:ring-2 focus:ring-blue-500"
                    }
                    button {
                        r#type: "submit",
                        class: "px-6 py-3 bg-blue-600 text-white rounded-lg hover:bg-blue-700 transition-colors font-medium disabled:opacity-50",
                        disabled: is_searching() || query().trim().is_empty(),
                        if is_searching() { "Searching..." } else { "Search" }
                    }
                }
            }

            // Results
            main {
                class: "max-w-4xl mx-auto px-4 py-6",

                if let Some(err) = error() {
                    div {
                        class: "bg-red-50 border border-red-200 text-red-700 p-4 rounded-lg mb-6",
                        "{err}"
                    }
                }

                if is_searching() {
                    div {
                        class: "text-center py-12",
                        div { class: "inline-flex space-x-2 mb-4",
                            div { class: "w-3 h-3 bg-blue-400 rounded-full animate-bounce" }
                            div { class: "w-3 h-3 bg-blue-400 rounded-full animate-bounce", style: "animation-delay: 0.1s" }
                            div { class: "w-3 h-3 bg-blue-400 rounded-full animate-bounce", style: "animation-delay: 0.2s" }
                        }
                        p { class: "text-gray-500", "Searching..." }
                    }
                } else if !results().is_empty() {
                    div {
                        class: "space-y-4",
                        p {
                            class: "text-sm text-gray-500 mb-4",
                            "Found {results().len()} result"
                            if results().len() != 1 { "s" }
                        }
                        for result in results() {
                            OrganizationCard { result: result.clone() }
                        }
                    }
                } else if !query().is_empty() {
                    div {
                        class: "text-center py-12",
                        p { class: "text-gray-500", "No organizations found matching your search." }
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct OrganizationCardProps {
    result: OrganizationMatch,
}

#[component]
fn OrganizationCard(props: OrganizationCardProps) -> Element {
    let org = &props.result.organization;
    let score = (props.result.similarity_score * 100.0) as i32;

    rsx! {
        div {
            class: "bg-white border border-gray-200 rounded-lg p-6 hover:shadow-md transition-shadow",

            div {
                class: "flex items-start justify-between mb-3",
                h3 {
                    class: "text-lg font-semibold text-gray-900",
                    "{org.name}"
                }
                span {
                    class: "text-sm text-gray-500 bg-gray-100 px-2 py-1 rounded",
                    "{score}% match"
                }
            }

            if let Some(description) = &org.description {
                p {
                    class: "text-gray-600 text-sm mb-4 line-clamp-2",
                    "{description}"
                }
            }

            div {
                class: "flex flex-wrap gap-3 text-sm",
                if let Some(phone) = &org.phone {
                    a {
                        href: "tel:{phone}",
                        class: "inline-flex items-center gap-1 text-blue-600 hover:text-blue-700",
                        "\u{1F4DE} {phone}"
                    }
                }
                if let Some(website) = &org.website {
                    a {
                        href: "{website}",
                        target: "_blank",
                        rel: "noopener noreferrer",
                        class: "inline-flex items-center gap-1 text-blue-600 hover:text-blue-700",
                        "\u{1F310} Website"
                    }
                }
                if let Some(address) = &org.primary_address {
                    span {
                        class: "text-gray-500",
                        "\u{1F4CD} {address}"
                    }
                }
            }
        }
    }
}

async fn do_search(
    query: String,
    results: &mut Signal<Vec<OrganizationMatch>>,
    is_searching: &mut Signal<bool>,
    error: &mut Signal<Option<String>>,
) {
    is_searching.set(true);
    error.set(None);

    match search_organizations(query).await {
        Ok(r) => results.set(r),
        Err(e) => error.set(Some(e.to_string())),
    }

    is_searching.set(false);
}

#[server]
async fn search_organizations(query: String) -> Result<Vec<OrganizationMatch>, ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    struct Variables {
        query: String,
        limit: i32,
    }

    let response: SearchOrganizationsResponse = client
        .query(
            SEARCH_ORGANIZATIONS,
            Some(Variables { query, limit: 20 }),
        )
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(response.search_organizations_semantic)
}
