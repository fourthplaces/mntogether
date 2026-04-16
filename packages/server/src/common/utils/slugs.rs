/// Convert a county name like "Scott" or "Lac qui Parle" to its
/// `service_area` tag slug: "scott-county" / "lac-qui-parle-county".
///
/// Used by the layout engine and post eligibility queries to match
/// `service_area` tags against a county record. The slug format must
/// stay in sync with the values seeded in `tags.json`.
pub fn county_service_area_slug(county_name: &str) -> String {
    let kebab = county_name
        .to_lowercase()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join("-");
    let kebab = kebab.replace('.', "").replace(',', "");
    format!("{}-county", kebab)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_county() {
        assert_eq!(county_service_area_slug("Scott"), "scott-county");
    }

    #[test]
    fn multi_word_county() {
        assert_eq!(
            county_service_area_slug("Lac qui Parle"),
            "lac-qui-parle-county"
        );
    }

    #[test]
    fn county_with_period() {
        assert_eq!(county_service_area_slug("St. Louis"), "st-louis-county");
    }

    #[test]
    fn county_with_dash_in_name() {
        assert_eq!(
            county_service_area_slug("Yellow Medicine"),
            "yellow-medicine-county"
        );
    }
}
