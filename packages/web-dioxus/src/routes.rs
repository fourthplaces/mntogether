//! Route definitions for the application

use dioxus::prelude::*;

use crate::pages::admin::{
    AdminDashboard, AdminExtraction, AdminLogin, AdminOrganizationDetail, AdminOrganizations,
    AdminPostDetail, AdminPosts, AdminResourceDetail, AdminResources, AdminWebsiteDetail,
    AdminWebsites,
};
use crate::pages::public::{Home, Search, Submit};
use crate::components::AdminLayout;

/// All application routes
#[derive(Clone, Debug, PartialEq, Routable)]
#[rustfmt::skip]
pub enum Route {
    // Public routes
    #[route("/")]
    Home {},

    #[route("/search")]
    Search {},

    #[route("/submit")]
    Submit {},

    // Admin routes
    #[route("/admin/login")]
    AdminLogin {},

    #[nest("/admin")]
        #[layout(AdminLayout)]
            #[route("/dashboard")]
            AdminDashboard {},

            #[route("/posts")]
            AdminPosts {},

            #[route("/posts/:id")]
            AdminPostDetail { id: String },

            #[route("/websites")]
            AdminWebsites {},

            #[route("/websites/:id")]
            AdminWebsiteDetail { id: String },

            #[route("/organizations")]
            AdminOrganizations {},

            #[route("/organizations/:id")]
            AdminOrganizationDetail { id: String },

            #[route("/resources")]
            AdminResources {},

            #[route("/resources/:id")]
            AdminResourceDetail { id: String },

            #[route("/extraction")]
            AdminExtraction {},
        #[end_layout]
    #[end_nest]
}
