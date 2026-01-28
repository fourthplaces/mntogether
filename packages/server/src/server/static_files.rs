use axum::{
    http::{header, StatusCode, Uri},
    response::{IntoResponse, Response},
};
use rust_embed::RustEmbed;

// Embed admin-spa build at compile time
// Run `cd packages/admin-spa && npm run build` before building the server
#[derive(RustEmbed)]
#[folder = "../admin-spa/dist"]
pub struct AdminAssets;

/// Serve static files from embedded assets with SPA fallback
pub async fn serve_admin(uri: Uri) -> Response {
    serve_spa::<AdminAssets>(uri, "/admin").await
}

/// Generic SPA serving function with fallback to index.html
async fn serve_spa<E: RustEmbed>(uri: Uri, base_path: &str) -> Response {
    let path = uri
        .path()
        .trim_start_matches(base_path)
        .trim_start_matches('/');

    // If path is empty, serve index.html
    let path = if path.is_empty() { "index.html" } else { path };

    match E::get(path) {
        Some(content) => {
            let mime = mime_guess::from_path(path).first_or_octet_stream();
            ([(header::CONTENT_TYPE, mime.as_ref())], content.data).into_response()
        }
        None => {
            // SPA fallback: if file not found, serve index.html
            // This allows client-side routing to work
            match E::get("index.html") {
                Some(content) => {
                    ([(header::CONTENT_TYPE, "text/html")], content.data).into_response()
                }
                None => (StatusCode::NOT_FOUND, "404 Not Found").into_response(),
            }
        }
    }
}
