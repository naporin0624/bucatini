use serde::{Deserialize, Serialize};

/// Wire format for `assets/third-party-licenses.json`. Variant declaration order is
/// load-bearing: `sort.rs` derives `Ord` from it so library entries sort before the
/// font/native appendix entries.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseCategory {
    Library,
    Font,
    Native,
}

/// One row in `assets/third-party-licenses.json`. `version`/`homepage` are `None` for
/// hand-written appendix entries that have neither (e.g. the NDI runtime notice).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LicenseEntry {
    pub name: String,
    pub version: Option<String>,
    pub license: String,
    pub text: String,
    pub homepage: Option<String>,
    pub category: LicenseCategory,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn category_serializes_to_lowercase_tag() {
        assert_eq!(
            serde_json::to_string(&LicenseCategory::Library).unwrap(),
            "\"library\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseCategory::Font).unwrap(),
            "\"font\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseCategory::Native).unwrap(),
            "\"native\""
        );
    }

    #[test]
    fn category_declared_order_is_library_then_font_then_native() {
        assert!(LicenseCategory::Library < LicenseCategory::Font);
        assert!(LicenseCategory::Font < LicenseCategory::Native);
    }

    #[test]
    fn entry_round_trips_through_json_with_null_optionals() {
        let entry = LicenseEntry {
            name: "LINE Seed JP".to_string(),
            version: None,
            license: "OFL-1.1".to_string(),
            text: "full text".to_string(),
            homepage: Some("https://seed.line.me/".to_string()),
            category: LicenseCategory::Font,
        };
        let json = serde_json::to_string(&entry).unwrap();
        assert!(json.contains("\"version\":null"));
        let round_tripped: LicenseEntry = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, entry);
    }
}
