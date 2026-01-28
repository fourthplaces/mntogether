use anyhow::{anyhow, Result};
use serde::Deserialize;
use tracing::{debug, error, instrument, warn};

/// Nominatim API response for geocoding
#[derive(Debug, Deserialize)]
struct NominatimResponse {
    lat: String,
    lon: String,
    display_name: String,
}

/// Geocoded location with coarse precision
#[derive(Debug, Clone)]
pub struct GeocodedLocation {
    pub latitude: f64,
    pub longitude: f64,
    pub display_name: String,
}

/// Geocode a city/state to lat/lng coordinates using Nominatim (OpenStreetMap)
///
/// Returns coarsened coordinates (2 decimal places ≈ 1km precision) for privacy
///
/// # Arguments
/// * `city` - City name (e.g., "Minneapolis")
/// * `state` - State name or abbreviation (e.g., "Minnesota" or "MN")
///
/// # Example
/// ```
/// let location = geocode_city("Minneapolis", "MN").await?;
/// assert_eq!(location.latitude, 44.98); // Coarsened to city-level
/// ```
#[instrument]
pub async fn geocode_city(city: &str, state: &str) -> Result<GeocodedLocation> {
    let query = format!("{}, {}, USA", city.trim(), state.trim());
    let url = format!(
        "https://nominatim.openstreetmap.org/search?q={}&format=json&limit=1",
        urlencoding::encode(&query)
    );

    debug!("Geocoding location: {}", query);

    let client = reqwest::Client::new();
    let response: Vec<NominatimResponse> = client
        .get(&url)
        .header(
            "User-Agent",
            "MNDigitalAid/1.0 (Emergency Response Platform)",
        )
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await
        .map_err(|e| {
            error!(error = %e, city = %city, state = %state, "Geocoding API request failed");
            anyhow!("Geocoding API request failed: {}", e)
        })?
        .json()
        .await
        .map_err(|e| {
            error!(error = %e, "Failed to parse geocoding response");
            anyhow!("Failed to parse geocoding response: {}", e)
        })?;

    let result = response.first().ok_or_else(|| {
        warn!(city = %city, state = %state, "Location not found by geocoding API");
        anyhow!("Location not found: {}", query)
    })?;

    let lat: f64 = result
        .lat
        .parse()
        .map_err(|e| anyhow!("Invalid latitude in response: {}", e))?;
    let lng: f64 = result
        .lon
        .parse()
        .map_err(|e| anyhow!("Invalid longitude in response: {}", e))?;

    // Coarsen coordinates for privacy (city-level precision)
    let (coarse_lat, coarse_lng) = coarsen_coords(lat, lng);

    debug!(
        "Geocoded {} → ({}, {}) [coarsened from ({}, {})]",
        query, coarse_lat, coarse_lng, lat, lng
    );

    Ok(GeocodedLocation {
        latitude: coarse_lat,
        longitude: coarse_lng,
        display_name: result.display_name.clone(),
    })
}

/// Coarsen coordinates to city-level precision for privacy
///
/// Rounds to 2 decimal places ≈ 1km precision (city area, not exact address)
///
/// # Arguments
/// * `lat` - Precise latitude
/// * `lng` - Precise longitude
///
/// # Returns
/// Tuple of (coarsened_lat, coarsened_lng)
///
/// # Example
/// ```
/// let (lat, lng) = coarsen_coords(44.977753, -93.265011);
/// assert_eq!(lat, 44.98);
/// assert_eq!(lng, -93.27);
/// ```
pub fn coarsen_coords(lat: f64, lng: f64) -> (f64, f64) {
    ((lat * 100.0).round() / 100.0, (lng * 100.0).round() / 100.0)
}

/// Calculate distance between two coordinates in kilometers
///
/// Uses Haversine formula for accuracy on Earth's surface
///
/// # Arguments
/// * `lat1`, `lng1` - First coordinate
/// * `lat2`, `lng2` - Second coordinate
///
/// # Returns
/// Distance in kilometers
pub fn calculate_distance_km(lat1: f64, lng1: f64, lat2: f64, lng2: f64) -> f64 {
    const EARTH_RADIUS_KM: f64 = 6371.0;

    let dlat = (lat2 - lat1).to_radians();
    let dlng = (lng2 - lng1).to_radians();

    let a = (dlat / 2.0).sin().powi(2)
        + lat1.to_radians().cos() * lat2.to_radians().cos() * (dlng / 2.0).sin().powi(2);

    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());

    EARTH_RADIUS_KM * c
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_coarsen_coords() {
        // Precise coords → city-level
        let (lat, lng) = coarsen_coords(44.977753, -93.265011);
        assert_eq!(lat, 44.98);
        assert_eq!(lng, -93.27);

        // Already coarse
        let (lat, lng) = coarsen_coords(44.98, -93.27);
        assert_eq!(lat, 44.98);
        assert_eq!(lng, -93.27);

        // Negative coordinates
        let (lat, lng) = coarsen_coords(-33.8688, 151.2093); // Sydney
        assert_eq!(lat, -33.87);
        assert_eq!(lng, 151.21);
    }

    #[test]
    fn test_calculate_distance() {
        // Minneapolis to St. Paul (≈16 km)
        let minneapolis = (44.98, -93.27);
        let st_paul = (44.95, -93.09);

        let distance = calculate_distance_km(minneapolis.0, minneapolis.1, st_paul.0, st_paul.1);

        // Should be approximately 16 km
        assert!(distance > 15.0 && distance < 17.0);

        // Same location
        let distance = calculate_distance_km(44.98, -93.27, 44.98, -93.27);
        assert!(distance < 0.1);
    }

    #[tokio::test]
    async fn test_geocode_city() {
        // Integration test - requires internet
        // Skip in CI by checking for env var
        if std::env::var("SKIP_GEOCODING_TESTS").is_ok() {
            return;
        }

        let result = geocode_city("Minneapolis", "MN").await;
        assert!(result.is_ok());

        let location = result.unwrap();
        assert!(location.latitude > 44.0 && location.latitude < 45.0);
        assert!(location.longitude < -93.0 && location.longitude > -94.0);
        assert!(location.display_name.contains("Minneapolis"));
    }

    #[tokio::test]
    async fn test_geocode_invalid_city() {
        if std::env::var("SKIP_GEOCODING_TESTS").is_ok() {
            return;
        }

        let result = geocode_city("NonexistentCity123", "XX").await;
        assert!(result.is_err());
    }
}
