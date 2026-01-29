// Build script for server
// Frontend apps (web-app, web-next) are now built and served separately
// via Docker Compose or standalone, not embedded in the server binary

fn main() {
    // No frontend builds - SPAs run independently
    println!("cargo:warning=Frontend apps are served separately (see docker-compose.yml)");
}
