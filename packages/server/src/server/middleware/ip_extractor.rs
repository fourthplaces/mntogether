use axum::{
    extract::{ConnectInfo, Request},
    middleware::Next,
    response::Response,
};
use std::net::{IpAddr, SocketAddr};

/// Extension key for storing extracted IP address
#[derive(Clone, Debug)]
pub struct ClientIp(pub IpAddr);

/// Middleware to extract client IP address from request
///
/// Priority:
/// 1. X-Forwarded-For header (for requests through proxies)
/// 2. X-Real-IP header (for Nginx)
/// 3. ConnectInfo socket address (direct connection)
pub async fn extract_client_ip(
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    mut request: Request,
    next: Next,
) -> Response {
    // Try X-Forwarded-For header first (comma-separated list, take first)
    let ip = if let Some(forwarded) = request.headers().get("x-forwarded-for") {
        forwarded
            .to_str()
            .ok()
            .and_then(|s| s.split(',').next())
            .and_then(|s| s.trim().parse::<IpAddr>().ok())
    } else if let Some(real_ip) = request.headers().get("x-real-ip") {
        // Try X-Real-IP header
        real_ip.to_str().ok().and_then(|s| s.parse::<IpAddr>().ok())
    } else {
        // Fall back to socket address
        Some(addr.ip())
    };

    // Store in request extensions
    if let Some(ip) = ip {
        request.extensions_mut().insert(ClientIp(ip));
    }

    next.run(request).await
}
