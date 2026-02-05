//! Admin extraction tools page

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, INGEST_SITE, TRIGGER_EXTRACTION};

/// Admin extraction tools page
#[component]
pub fn AdminExtraction() -> Element {
    // Site Ingest state
    let mut ingest_url = use_signal(String::new);
    let mut max_pages = use_signal(|| 10);
    let mut ingest_loading = use_signal(|| false);
    let mut ingest_result = use_signal(|| None::<String>);

    // Query Extraction state
    let mut query = use_signal(String::new);
    let mut site_url = use_signal(String::new);
    let mut query_loading = use_signal(|| false);
    let mut query_result = use_signal(|| None::<String>);

    let handle_ingest = move |_| {
        let url = ingest_url().trim().to_string();
        if url.is_empty() {
            return;
        }

        let pages = max_pages();
        spawn(async move {
            ingest_loading.set(true);
            ingest_result.set(None);

            match ingest_site(url, pages).await {
                Ok(result) => ingest_result.set(Some(result)),
                Err(e) => ingest_result.set(Some(format!("Error: {}", e))),
            }

            ingest_loading.set(false);
        });
    };

    let handle_extraction = move |_| {
        let q = query().trim().to_string();
        if q.is_empty() {
            return;
        }

        let site = site_url().trim().to_string();
        spawn(async move {
            query_loading.set(true);
            query_result.set(None);

            match trigger_extraction(q, if site.is_empty() { None } else { Some(site) }).await {
                Ok(result) => query_result.set(Some(result)),
                Err(e) => query_result.set(Some(format!("Error: {}", e))),
            }

            query_loading.set(false);
        });
    };

    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Extraction Tools" }

            div {
                class: "grid grid-cols-1 lg:grid-cols-2 gap-6",

                // Site Ingest Card
                div {
                    class: "bg-white rounded-lg shadow-sm border border-gray-200 p-6",
                    h2 { class: "text-lg font-semibold text-gray-900 mb-4", "Site Ingest" }
                    p { class: "text-sm text-gray-600 mb-4", "Crawl and ingest pages from a website." }

                    div {
                        class: "space-y-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Site URL" }
                            input {
                                r#type: "url",
                                value: "{ingest_url}",
                                oninput: move |e| ingest_url.set(e.value()),
                                placeholder: "https://example.org",
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Max Pages" }
                            input {
                                r#type: "number",
                                value: "{max_pages}",
                                oninput: move |e| {
                                    if let Ok(n) = e.value().parse() {
                                        max_pages.set(n);
                                    }
                                },
                                min: "1",
                                max: "100",
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
                            }
                        }
                        button {
                            class: "w-full py-2 bg-amber-600 text-white rounded-md hover:bg-amber-700 disabled:opacity-50",
                            disabled: ingest_loading() || ingest_url().trim().is_empty(),
                            onclick: handle_ingest,
                            if ingest_loading() { "Ingesting..." } else { "Start Ingest" }
                        }
                    }

                    if let Some(result) = ingest_result() {
                        pre {
                            class: "mt-4 p-3 bg-gray-50 rounded text-xs overflow-auto max-h-48",
                            "{result}"
                        }
                    }
                }

                // Query Extraction Card
                div {
                    class: "bg-white rounded-lg shadow-sm border border-gray-200 p-6",
                    h2 { class: "text-lg font-semibold text-gray-900 mb-4", "Query Extraction" }
                    p { class: "text-sm text-gray-600 mb-4", "Extract information using a natural language query." }

                    div {
                        class: "space-y-4",
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Query" }
                            input {
                                r#type: "text",
                                value: "{query}",
                                oninput: move |e| query.set(e.value()),
                                placeholder: "e.g., Find food assistance programs",
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
                            }
                        }
                        div {
                            label { class: "block text-sm font-medium text-gray-700 mb-1", "Site URL (optional)" }
                            input {
                                r#type: "url",
                                value: "{site_url}",
                                oninput: move |e| site_url.set(e.value()),
                                placeholder: "https://example.org (optional)",
                                class: "w-full px-3 py-2 border border-gray-300 rounded-md focus:outline-none focus:ring-2 focus:ring-amber-500"
                            }
                        }
                        button {
                            class: "w-full py-2 bg-amber-600 text-white rounded-md hover:bg-amber-700 disabled:opacity-50",
                            disabled: query_loading() || query().trim().is_empty(),
                            onclick: handle_extraction,
                            if query_loading() { "Extracting..." } else { "Run Extraction" }
                        }
                    }

                    if let Some(result) = query_result() {
                        pre {
                            class: "mt-4 p-3 bg-gray-50 rounded text-xs overflow-auto max-h-48",
                            "{result}"
                        }
                    }
                }
            }
        }
    }
}

#[server]
async fn ingest_site(site_url: String, max_pages: i32) -> Result<String, ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    #[serde(rename_all = "camelCase")]
    struct Variables {
        site_url: String,
        max_pages: i32,
    }

    let result: serde_json::Value = client
        .mutate(INGEST_SITE, Some(Variables { site_url, max_pages }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}

#[server]
async fn trigger_extraction(query: String, site: Option<String>) -> Result<String, ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    struct Input {
        query: String,
        site: Option<String>,
    }

    #[derive(serde::Serialize)]
    struct Variables {
        input: Input,
    }

    let result: serde_json::Value = client
        .mutate(TRIGGER_EXTRACTION, Some(Variables { input: Input { query, site } }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(serde_json::to_string_pretty(&result).unwrap_or_default())
}
