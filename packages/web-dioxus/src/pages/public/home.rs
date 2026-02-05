//! Home page component

use dioxus::prelude::*;

use crate::components::{PostCard, PostCardSkeleton};
use crate::graphql::{GraphQLClient, GET_PUBLISHED_POSTS};
use crate::routes::Route;
use crate::state::PostFilter;
use crate::types::{GetPublishedPostsResponse, Post, PostType};

/// Home page - displays published posts with filtering
#[component]
pub fn Home() -> Element {
    // Fetch posts on server and client
    let posts = use_server_future(fetch_published_posts)?;

    let mut search_query = use_signal(String::new);
    let mut active_filter = use_signal(|| PostFilter::All);

    // Derive filtered posts
    let filtered_posts = use_memo(move || {
        let posts = match posts.value().as_ref() {
            Some(Ok(p)) => p.clone(),
            _ => vec![],
        };

        posts
            .into_iter()
            .filter(|post| {
                // Filter by post type
                match active_filter() {
                    PostFilter::All => true,
                    PostFilter::Service => post.post_type == Some(PostType::Service),
                    PostFilter::Opportunity => post.post_type == Some(PostType::Opportunity),
                    PostFilter::Business => post.post_type == Some(PostType::Business),
                }
            })
            .filter(|post| {
                // Search filter
                let query = search_query().to_lowercase();
                if query.is_empty() {
                    return true;
                }

                let title = post.title.to_lowercase();
                let org = post.organization_name.to_lowercase();
                let tldr = post.tldr.as_deref().unwrap_or("").to_lowercase();
                let description = post.description.to_lowercase();
                let location = post.location.as_deref().unwrap_or("").to_lowercase();
                let category = post.category.as_deref().unwrap_or("").to_lowercase();

                title.contains(&query)
                    || org.contains(&query)
                    || tldr.contains(&query)
                    || description.contains(&query)
                    || location.contains(&query)
                    || category.contains(&query)
            })
            .collect::<Vec<_>>()
    });

    // Count posts by type
    let post_counts = use_memo(move || {
        let posts = match posts.value().as_ref() {
            Some(Ok(p)) => p.clone(),
            _ => vec![],
        };

        let mut counts = std::collections::HashMap::new();
        counts.insert(PostFilter::All, posts.len());
        counts.insert(
            PostFilter::Service,
            posts.iter().filter(|p| p.post_type == Some(PostType::Service)).count(),
        );
        counts.insert(
            PostFilter::Opportunity,
            posts.iter().filter(|p| p.post_type == Some(PostType::Opportunity)).count(),
        );
        counts.insert(
            PostFilter::Business,
            posts.iter().filter(|p| p.post_type == Some(PostType::Business)).count(),
        );
        counts
    });

    let is_loading = posts.value().is_none();
    let error = posts.value().as_ref().and_then(|r| r.as_ref().err());

    rsx! {
        div {
            class: "min-h-screen bg-gradient-to-b from-blue-50 to-white",

            // Hero Section
            header {
                class: "bg-white border-b border-gray-100",
                div {
                    class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8 sm:py-12",
                    div {
                        class: "text-center max-w-3xl mx-auto",
                        h1 {
                            class: "text-4xl sm:text-5xl font-bold text-gray-900 mb-4",
                            "MN Together"
                        }
                        p {
                            class: "text-lg sm:text-xl text-gray-600 mb-8",
                            "Connecting Minnesota communities with services, volunteer opportunities, and local businesses making a difference."
                        }

                        // Search Bar
                        div {
                            class: "relative max-w-xl mx-auto mb-6",
                            div {
                                class: "absolute inset-y-0 left-0 pl-4 flex items-center pointer-events-none",
                                svg {
                                    class: "h-5 w-5 text-gray-400",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                                    }
                                }
                            }
                            input {
                                r#type: "text",
                                placeholder: "Search by name, location, or keyword...",
                                value: "{search_query}",
                                oninput: move |e| search_query.set(e.value()),
                                class: "w-full pl-12 pr-4 py-3.5 bg-gray-50 border border-gray-200 rounded-xl text-gray-900 placeholder-gray-500 focus:outline-none focus:ring-2 focus:ring-blue-500 focus:border-transparent transition-all"
                            }
                            if !search_query().is_empty() {
                                button {
                                    class: "absolute inset-y-0 right-0 pr-4 flex items-center text-gray-400 hover:text-gray-600",
                                    onclick: move |_| search_query.set(String::new()),
                                    svg {
                                        class: "h-5 w-5",
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

                        // Submit CTA
                        Link {
                            to: Route::Submit {},
                            class: "inline-flex items-center gap-2 px-6 py-3 bg-blue-600 text-white rounded-xl hover:bg-blue-700 transition-colors font-medium shadow-sm hover:shadow-md",
                            svg {
                                class: "w-5 h-5",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M12 4v16m8-8H4"
                                }
                            }
                            "Submit a Resource"
                        }
                    }
                }
            }

            // Filter Tabs
            div {
                class: "bg-white border-b border-gray-100 sticky top-0 z-10",
                div {
                    class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8",
                    div {
                        class: "flex items-center gap-1 overflow-x-auto py-3 -mx-4 px-4 sm:mx-0 sm:px-0",
                        for filter in PostFilter::variants() {
                            {
                                let filter = *filter;
                                let is_active = active_filter() == filter;
                                let count = post_counts().get(&filter).copied().unwrap_or(0);
                                rsx! {
                                    button {
                                        key: "{filter:?}",
                                        class: if is_active {
                                            "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all bg-blue-100 text-blue-700"
                                        } else {
                                            "flex items-center gap-2 px-4 py-2 rounded-lg text-sm font-medium whitespace-nowrap transition-all bg-gray-50 text-gray-600 hover:bg-gray-100"
                                        },
                                        onclick: move |_| active_filter.set(filter),
                                        span { "{filter.icon()}" }
                                        "{filter.label()}"
                                        if count > 0 {
                                            span {
                                                class: if is_active {
                                                    "ml-1 px-2 py-0.5 rounded-full text-xs bg-blue-200 text-blue-800"
                                                } else {
                                                    "ml-1 px-2 py-0.5 rounded-full text-xs bg-gray-200 text-gray-600"
                                                },
                                                "{count}"
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }

            // Main Content
            main {
                class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8",

                // Loading State
                if is_loading {
                    div {
                        class: "grid gap-6 sm:grid-cols-2 lg:grid-cols-3",
                        for i in 0..6 {
                            PostCardSkeleton { key: "{i}" }
                        }
                    }
                }

                // Error State
                else if let Some(err) = error {
                    div {
                        class: "text-center py-12",
                        div {
                            class: "inline-flex items-center justify-center w-16 h-16 rounded-full bg-red-100 mb-4",
                            svg {
                                class: "w-8 h-8 text-red-600",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M12 9v2m0 4h.01m-6.938 4h13.856c1.54 0 2.502-1.667 1.732-3L13.732 4c-.77-1.333-2.694-1.333-3.464 0L3.34 16c-.77 1.333.192 3 1.732 3z"
                                }
                            }
                        }
                        h3 { class: "text-lg font-medium text-gray-900 mb-2", "Unable to load resources" }
                        p { class: "text-gray-500 mb-4", "{err}" }
                    }
                }

                // Empty State
                else if filtered_posts().is_empty() {
                    div {
                        class: "text-center py-16",
                        div {
                            class: "inline-flex items-center justify-center w-20 h-20 rounded-full bg-gray-100 mb-6",
                            svg {
                                class: "w-10 h-10 text-gray-400",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M21 21l-6-6m2-5a7 7 0 11-14 0 7 7 0 0114 0z"
                                }
                            }
                        }
                        if !search_query().is_empty() {
                            h3 { class: "text-xl font-semibold text-gray-900 mb-2", "No results found" }
                            p {
                                class: "text-gray-500 mb-6 max-w-md mx-auto",
                                "We couldn't find any resources matching \"{search_query}\". Try adjusting your search or filters."
                            }
                            button {
                                class: "px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors",
                                onclick: move |_| {
                                    search_query.set(String::new());
                                    active_filter.set(PostFilter::All);
                                },
                                "Clear Filters"
                            }
                        } else {
                            h3 { class: "text-xl font-semibold text-gray-900 mb-2", "No resources yet" }
                            p {
                                class: "text-gray-500 mb-6 max-w-md mx-auto",
                                "Be the first to share a resource with the community!"
                            }
                            Link {
                                to: Route::Submit {},
                                class: "inline-flex items-center gap-2 px-6 py-3 bg-blue-600 text-white rounded-xl hover:bg-blue-700 transition-colors font-medium",
                                svg {
                                    class: "w-5 h-5",
                                    fill: "none",
                                    stroke: "currentColor",
                                    view_box: "0 0 24 24",
                                    path {
                                        stroke_linecap: "round",
                                        stroke_linejoin: "round",
                                        stroke_width: "2",
                                        d: "M12 4v16m8-8H4"
                                    }
                                }
                                "Submit a Resource"
                            }
                        }
                    }
                }

                // Posts Grid
                else {
                    // Results count
                    div {
                        class: "mb-6 flex items-center justify-between",
                        p {
                            class: "text-sm text-gray-500",
                            "Showing "
                            span { class: "font-medium text-gray-900", "{filtered_posts().len()}" }
                            " resource"
                            if filtered_posts().len() != 1 { "s" }
                            if !search_query().is_empty() {
                                " for \""
                                span { class: "font-medium", "{search_query}" }
                                "\""
                            }
                        }
                    }

                    div {
                        class: "grid gap-6 sm:grid-cols-2 lg:grid-cols-3",
                        for post in filtered_posts() {
                            PostCard { key: "{post.id}", post: post.clone() }
                        }
                    }
                }
            }

            // Footer
            footer {
                class: "bg-white border-t border-gray-100 mt-12",
                div {
                    class: "max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8",
                    div {
                        class: "text-center",
                        h2 { class: "text-lg font-semibold text-gray-900 mb-2", "MN Together" }
                        p {
                            class: "text-gray-500 text-sm max-w-md mx-auto",
                            "Connecting resources with those who need them. Building stronger communities across Minnesota."
                        }
                    }
                }
            }
        }
    }
}

/// Server function to fetch published posts
#[server]
async fn fetch_published_posts() -> Result<Vec<Post>, ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Serialize)]
    struct Variables {
        limit: i32,
    }

    let response: GetPublishedPostsResponse = client
        .query(GET_PUBLISHED_POSTS, Some(Variables { limit: 100 }))
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    Ok(response.published_posts)
}
