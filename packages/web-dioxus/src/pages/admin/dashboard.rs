//! Admin dashboard page

use dioxus::prelude::*;

use crate::graphql::{GraphQLClient, GET_ADMIN_STATS};
use crate::routes::Route;
use crate::types::{ListingStatus, WebsiteStatus};

/// Admin dashboard with stats overview
#[component]
pub fn AdminDashboard() -> Element {
    let stats = use_server_future(fetch_admin_stats)?;

    let (website_stats, listing_stats) = match stats.value().as_ref() {
        Some(Ok(s)) => s.clone(),
        _ => (WebsiteStats::default(), ListingStats::default()),
    };

    rsx! {
        div {
            h1 { class: "text-2xl font-bold text-gray-900 mb-6", "Dashboard" }

            // Stats Grid
            div {
                class: "grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-6 mb-8",

                StatCard {
                    title: "Total Websites",
                    value: website_stats.total,
                    icon: "\u{1F310}",
                    color: "blue"
                }
                StatCard {
                    title: "Pending Websites",
                    value: website_stats.pending,
                    icon: "\u{23F3}",
                    color: "amber"
                }
                StatCard {
                    title: "Total Listings",
                    value: listing_stats.total,
                    icon: "\u{1F4CB}",
                    color: "green"
                }
                StatCard {
                    title: "Pending Approval",
                    value: listing_stats.pending,
                    icon: "\u{2705}",
                    color: "orange"
                }
            }

            // Quick Actions
            div {
                class: "bg-white rounded-lg shadow-sm border border-gray-200 p-6",
                h2 { class: "text-lg font-semibold text-gray-900 mb-4", "Quick Actions" }
                div {
                    class: "flex flex-wrap gap-3",
                    QuickActionLink {
                        to: Route::AdminPosts {},
                        label: "Review Posts",
                        icon: "\u{1F4DD}"
                    }
                    QuickActionLink {
                        to: Route::AdminWebsites {},
                        label: "Manage Websites",
                        icon: "\u{1F310}"
                    }
                    QuickActionLink {
                        to: Route::AdminResources {},
                        label: "View Resources",
                        icon: "\u{1F4DA}"
                    }
                    QuickActionLink {
                        to: Route::AdminExtraction {},
                        label: "Extraction Tools",
                        icon: "\u{1F527}"
                    }
                }
            }
        }
    }
}

#[derive(Clone, Default)]
struct WebsiteStats {
    total: i32,
    pending: i32,
    approved: i32,
}

#[derive(Clone, Default)]
struct ListingStats {
    total: i32,
    pending: i32,
    active: i32,
}

#[derive(Props, Clone, PartialEq)]
struct StatCardProps {
    title: &'static str,
    value: i32,
    icon: &'static str,
    color: &'static str,
}

#[component]
fn StatCard(props: StatCardProps) -> Element {
    let bg_class = match props.color {
        "blue" => "bg-blue-50",
        "amber" => "bg-amber-50",
        "green" => "bg-green-50",
        "orange" => "bg-orange-50",
        _ => "bg-gray-50",
    };

    let text_class = match props.color {
        "blue" => "text-blue-700",
        "amber" => "text-amber-700",
        "green" => "text-green-700",
        "orange" => "text-orange-700",
        _ => "text-gray-700",
    };

    rsx! {
        div {
            class: "bg-white rounded-lg shadow-sm border border-gray-200 p-6",
            div {
                class: "flex items-center justify-between",
                div {
                    p { class: "text-sm text-gray-500", "{props.title}" }
                    p { class: "text-3xl font-bold text-gray-900 mt-1", "{props.value}" }
                }
                div {
                    class: "w-12 h-12 rounded-full {bg_class} {text_class} flex items-center justify-center text-2xl",
                    "{props.icon}"
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct QuickActionLinkProps {
    to: Route,
    label: &'static str,
    icon: &'static str,
}

#[component]
fn QuickActionLink(props: QuickActionLinkProps) -> Element {
    rsx! {
        Link {
            to: props.to.clone(),
            class: "inline-flex items-center gap-2 px-4 py-2 bg-gray-100 text-gray-700 rounded-lg hover:bg-gray-200 transition-colors",
            span { "{props.icon}" }
            "{props.label}"
        }
    }
}

#[server]
async fn fetch_admin_stats() -> Result<(WebsiteStats, ListingStats), ServerFnError> {
    let url = std::env::var("API_URL").unwrap_or_else(|_| "http://localhost:8080/graphql".to_string());
    let client = GraphQLClient::new(url);

    #[derive(serde::Deserialize)]
    struct Website {
        id: String,
        status: WebsiteStatus,
    }

    #[derive(serde::Deserialize)]
    struct Listing {
        id: String,
        status: ListingStatus,
    }

    #[derive(serde::Deserialize)]
    struct Response {
        websites: Vec<Website>,
        listings: Vec<Listing>,
    }

    let response: Response = client
        .query::<(), Response>(GET_ADMIN_STATS, None)
        .await
        .map_err(|e| ServerFnError::new(e.to_string()))?;

    let website_stats = WebsiteStats {
        total: response.websites.len() as i32,
        pending: response
            .websites
            .iter()
            .filter(|w| w.status == WebsiteStatus::PendingReview)
            .count() as i32,
        approved: response
            .websites
            .iter()
            .filter(|w| w.status == WebsiteStatus::Approved)
            .count() as i32,
    };

    let listing_stats = ListingStats {
        total: response.listings.len() as i32,
        pending: response
            .listings
            .iter()
            .filter(|l| l.status == ListingStatus::PendingApproval)
            .count() as i32,
        active: response
            .listings
            .iter()
            .filter(|l| l.status == ListingStatus::Active)
            .count() as i32,
    };

    Ok((website_stats, listing_stats))
}
