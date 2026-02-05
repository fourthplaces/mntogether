//! Admin navigation component

use dioxus::prelude::*;

use crate::auth::{logout, use_auth};
use crate::routes::Route;

/// Admin navigation bar
#[component]
pub fn AdminNav() -> Element {
    let auth = use_auth();
    let navigator = use_navigator();

    let handle_logout = move |_| {
        spawn(async move {
            if logout().await.is_ok() {
                navigator.push(Route::AdminLogin {});
            }
        });
    };

    rsx! {
        nav {
            class: "bg-white border-b border-gray-200 px-6 py-3",
            div {
                class: "flex items-center justify-between",

                // Logo / Brand
                div {
                    class: "flex items-center gap-6",
                    Link {
                        to: Route::AdminDashboard {},
                        class: "text-xl font-bold text-amber-700",
                        "MN Together Admin"
                    }

                    // Nav links
                    div {
                        class: "hidden md:flex items-center gap-1",
                        NavLink { to: Route::AdminDashboard {}, label: "Dashboard" }
                        NavLink { to: Route::AdminPosts {}, label: "Posts" }
                        NavLink { to: Route::AdminWebsites {}, label: "Websites" }
                        NavLink { to: Route::AdminOrganizations {}, label: "Organizations" }
                        NavLink { to: Route::AdminResources {}, label: "Resources" }
                        NavLink { to: Route::AdminExtraction {}, label: "Extraction" }
                    }
                }

                // User menu
                div {
                    class: "flex items-center gap-4",
                    if let Some(user) = auth.user.read().as_ref() {
                        span {
                            class: "text-sm text-gray-600",
                            "{user.phone_number}"
                        }
                    }
                    button {
                        class: "text-sm text-gray-600 hover:text-gray-900 px-3 py-1.5 rounded hover:bg-gray-100",
                        onclick: handle_logout,
                        "Logout"
                    }
                }
            }
        }
    }
}

#[derive(Props, Clone, PartialEq)]
struct NavLinkProps {
    to: Route,
    label: &'static str,
}

#[component]
fn NavLink(props: NavLinkProps) -> Element {
    let route = use_route::<Route>();
    let is_active = route == props.to;

    rsx! {
        Link {
            to: props.to.clone(),
            class: if is_active {
                "px-3 py-2 rounded-md text-sm font-medium bg-amber-100 text-amber-800"
            } else {
                "px-3 py-2 rounded-md text-sm font-medium text-gray-600 hover:bg-gray-100 hover:text-gray-900"
            },
            "{props.label}"
        }
    }
}
