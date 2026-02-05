//! Post card component

use dioxus::prelude::*;

use crate::types::{CapacityStatus, Post, PostType, Urgency};

/// Props for PostCard
#[derive(Props, Clone, PartialEq)]
pub struct PostCardProps {
    pub post: Post,
}

/// Post card component displaying a single post
#[component]
pub fn PostCard(props: PostCardProps) -> Element {
    let post = &props.post;

    let urgency_styles = get_urgency_styles(post.urgency);
    let post_type_styles = get_post_type_styles(post.post_type);
    let capacity_styles = get_capacity_styles(post.capacity_status);

    rsx! {
        div {
            class: "rounded-xl border {urgency_styles.border} {urgency_styles.bg} p-5 hover:shadow-lg transition-all duration-200 flex flex-col h-full",

            // Header: Post Type + Urgency
            div {
                class: "flex items-center justify-between mb-3",
                span {
                    class: "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-xs font-medium {post_type_styles.bg} {post_type_styles.text}",
                    span { "{post_type_styles.icon}" }
                    "{post_type_styles.label}"
                }
                div {
                    class: "flex items-center gap-2",
                    if let Some(styles) = capacity_styles {
                        span {
                            class: "px-2 py-0.5 rounded-full text-xs font-medium {styles.bg} {styles.text}",
                            "{styles.label}"
                        }
                    }
                    if let Some(urgency) = post.urgency {
                        if urgency != Urgency::Low {
                            span {
                                class: "px-2.5 py-1 rounded-full text-xs font-semibold {urgency_styles.badge}",
                                "{urgency:?}"
                            }
                        }
                    }
                }
            }

            // Title
            h3 {
                class: "text-lg font-semibold text-gray-900 mb-1 line-clamp-2",
                "{post.title}"
            }

            // Organization
            p {
                class: "text-sm font-medium text-gray-600 mb-2",
                "{post.organization_name}"
            }

            // Category + Location
            div {
                class: "flex flex-wrap items-center gap-2 text-sm text-gray-500 mb-3",
                if let Some(category) = &post.category {
                    span {
                        class: "inline-flex items-center gap-1 bg-gray-100 px-2 py-0.5 rounded text-xs",
                        "{category}"
                    }
                }
                if let Some(location) = &post.location {
                    span {
                        class: "inline-flex items-center gap-1",
                        // Location icon
                        svg {
                            class: "w-3.5 h-3.5",
                            fill: "none",
                            stroke: "currentColor",
                            view_box: "0 0 24 24",
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M17.657 16.657L13.414 20.9a1.998 1.998 0 01-2.827 0l-4.244-4.243a8 8 0 1111.314 0z"
                            }
                            path {
                                stroke_linecap: "round",
                                stroke_linejoin: "round",
                                stroke_width: "2",
                                d: "M15 11a3 3 0 11-6 0 3 3 0 016 0z"
                            }
                        }
                        "{location}"
                    }
                }
            }

            // TLDR / Description
            p {
                class: "text-gray-700 text-sm mb-4 line-clamp-3 flex-grow",
                if let Some(tldr) = &post.tldr {
                    "{tldr}"
                } else {
                    "{post.description}"
                }
            }

            // Footer: Source Link + Time
            div {
                class: "mt-auto pt-3 border-t border-gray-200/60",
                if let Some(source_url) = &post.source_url {
                    div {
                        class: "mb-2",
                        a {
                            href: "{source_url}",
                            target: "_blank",
                            rel: "noopener noreferrer",
                            class: "inline-flex items-center gap-1.5 px-3 py-1.5 bg-blue-600 text-white text-sm rounded-lg hover:bg-blue-700 transition-colors",
                            // External link icon
                            svg {
                                class: "w-4 h-4",
                                fill: "none",
                                stroke: "currentColor",
                                view_box: "0 0 24 24",
                                path {
                                    stroke_linecap: "round",
                                    stroke_linejoin: "round",
                                    stroke_width: "2",
                                    d: "M10 6H6a2 2 0 00-2 2v10a2 2 0 002 2h10a2 2 0 002-2v-4M14 4h6m0 0v6m0-6L10 14"
                                }
                            }
                            "Learn More"
                        }
                    }
                }
                p {
                    class: "text-xs text-gray-400",
                    "Posted {format_time_ago(&post.created_at)}"
                }
            }
        }
    }
}

/// Skeleton loader for posts
#[component]
pub fn PostCardSkeleton() -> Element {
    rsx! {
        div {
            class: "rounded-xl border border-gray-200 bg-white p-5 animate-pulse",
            div {
                class: "flex items-center justify-between mb-3",
                div { class: "h-6 w-20 bg-gray-200 rounded-full" }
                div { class: "h-6 w-16 bg-gray-200 rounded-full" }
            }
            div { class: "h-6 w-3/4 bg-gray-200 rounded mb-2" }
            div { class: "h-4 w-1/2 bg-gray-200 rounded mb-3" }
            div {
                class: "flex gap-2 mb-3",
                div { class: "h-5 w-16 bg-gray-200 rounded" }
                div { class: "h-5 w-24 bg-gray-200 rounded" }
            }
            div {
                class: "space-y-2 mb-4",
                div { class: "h-4 w-full bg-gray-200 rounded" }
                div { class: "h-4 w-5/6 bg-gray-200 rounded" }
            }
            div {
                class: "pt-3 border-t border-gray-100",
                div { class: "h-8 w-24 bg-gray-200 rounded-lg" }
            }
        }
    }
}

// Helper structs for styling
struct UrgencyStyles {
    bg: &'static str,
    border: &'static str,
    badge: &'static str,
}

struct PostTypeStyles {
    bg: &'static str,
    text: &'static str,
    icon: &'static str,
    label: &'static str,
}

struct CapacityStyles {
    bg: &'static str,
    text: &'static str,
    label: &'static str,
}

fn get_urgency_styles(urgency: Option<Urgency>) -> UrgencyStyles {
    match urgency {
        Some(Urgency::Urgent) => UrgencyStyles {
            bg: "bg-red-50",
            border: "border-red-200",
            badge: "bg-red-100 text-red-700",
        },
        Some(Urgency::High) => UrgencyStyles {
            bg: "bg-orange-50",
            border: "border-orange-200",
            badge: "bg-orange-100 text-orange-700",
        },
        Some(Urgency::Medium) => UrgencyStyles {
            bg: "bg-amber-50",
            border: "border-amber-200",
            badge: "bg-amber-100 text-amber-700",
        },
        _ => UrgencyStyles {
            bg: "bg-white",
            border: "border-gray-200",
            badge: "bg-gray-100 text-gray-700",
        },
    }
}

fn get_post_type_styles(post_type: Option<PostType>) -> PostTypeStyles {
    match post_type {
        Some(PostType::Service) => PostTypeStyles {
            bg: "bg-blue-100",
            text: "text-blue-700",
            icon: "\u{1F3E5}",
            label: "Service",
        },
        Some(PostType::Opportunity) => PostTypeStyles {
            bg: "bg-emerald-100",
            text: "text-emerald-700",
            icon: "\u{1F91D}",
            label: "Opportunity",
        },
        Some(PostType::Business) => PostTypeStyles {
            bg: "bg-purple-100",
            text: "text-purple-700",
            icon: "\u{1F3EA}",
            label: "Business",
        },
        Some(PostType::Professional) => PostTypeStyles {
            bg: "bg-indigo-100",
            text: "text-indigo-700",
            icon: "\u{1F464}",
            label: "Professional",
        },
        None => PostTypeStyles {
            bg: "bg-gray-100",
            text: "text-gray-700",
            icon: "\u{1F4CB}",
            label: "Resource",
        },
    }
}

fn get_capacity_styles(status: Option<CapacityStatus>) -> Option<CapacityStyles> {
    match status {
        Some(CapacityStatus::Accepting) => Some(CapacityStyles {
            bg: "bg-green-100",
            text: "text-green-700",
            label: "Accepting",
        }),
        Some(CapacityStatus::Paused) => Some(CapacityStyles {
            bg: "bg-yellow-100",
            text: "text-yellow-700",
            label: "Paused",
        }),
        Some(CapacityStatus::AtCapacity) => Some(CapacityStyles {
            bg: "bg-red-100",
            text: "text-red-700",
            label: "At Capacity",
        }),
        None => None,
    }
}

fn format_time_ago(date_string: &str) -> String {
    // Parse ISO date and calculate time ago
    // For now, just return a placeholder - in production use chrono
    if let Ok(date) = chrono::DateTime::parse_from_rfc3339(date_string) {
        let now = chrono::Utc::now();
        let diff = now.signed_duration_since(date);

        let days = diff.num_days();
        if days == 0 {
            "Today".to_string()
        } else if days == 1 {
            "Yesterday".to_string()
        } else if days < 7 {
            format!("{} days ago", days)
        } else if days < 30 {
            format!("{} weeks ago", days / 7)
        } else {
            format!("{} months ago", days / 30)
        }
    } else {
        "Recently".to_string()
    }
}
