use serde::Deserialize;

use crate::license_entry::{LicenseCategory, LicenseEntry};

/// Mirrors the JSON emitted by `cargo bundle-licenses --format json` (cargo-bundle-licenses
/// 4.2.0). `root_name` is intentionally omitted from this struct: serde ignores unknown JSON
/// fields by default, and it is not needed here.
#[derive(Debug, Deserialize)]
pub struct CargoBundleOutput {
    pub third_party_libraries: Vec<CargoBundleLicense>,
}

#[derive(Debug, Deserialize)]
pub struct CargoBundleLicense {
    pub package_name: String,
    pub package_version: String,
    #[serde(default)]
    pub repository: String,
    pub license: String,
    pub licenses: Vec<CargoBundleLicenseText>,
}

#[derive(Debug, Deserialize)]
pub struct CargoBundleLicenseText {
    pub license: String,
    pub text: String,
}

pub fn normalize(bundle: CargoBundleOutput) -> Vec<LicenseEntry> {
    bundle
        .third_party_libraries
        .into_iter()
        .map(normalize_one)
        .collect()
}

fn normalize_one(lib: CargoBundleLicense) -> LicenseEntry {
    let text = lib
        .licenses
        .iter()
        .map(|component| format!("### {}\n\n{}", component.license, component.text))
        .collect::<Vec<_>>()
        .join("\n\n---\n\n");
    let homepage = if lib.repository.is_empty() {
        None
    } else {
        Some(lib.repository)
    };

    LicenseEntry {
        name: lib.package_name,
        version: Some(lib.package_version),
        license: lib.license,
        text,
        homepage,
        category: LicenseCategory::Library,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_dual_license_package_and_joins_repository_as_homepage() {
        let raw = r#"{
            "root_name": "bucatini",
            "third_party_libraries": [
                {
                    "package_name": "eframe",
                    "package_version": "0.35.0",
                    "repository": "https://github.com/emilk/egui",
                    "license": "MIT OR Apache-2.0",
                    "licenses": [
                        {"license": "MIT", "text": "MIT license text"},
                        {"license": "Apache-2.0", "text": "Apache license text"}
                    ]
                },
                {
                    "package_name": "no-repo-crate",
                    "package_version": "1.0.0",
                    "repository": "",
                    "license": "MIT",
                    "licenses": [
                        {"license": "MIT", "text": "MIT license text"}
                    ]
                }
            ]
        }"#;
        let bundle: CargoBundleOutput = serde_json::from_str(raw).unwrap();

        let entries = normalize(bundle);

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "eframe");
        assert_eq!(entries[0].version, Some("0.35.0".to_string()));
        assert_eq!(entries[0].license, "MIT OR Apache-2.0");
        assert_eq!(
            entries[0].homepage,
            Some("https://github.com/emilk/egui".to_string())
        );
        assert_eq!(entries[0].category, LicenseCategory::Library);
        assert_eq!(
            entries[0].text,
            "### MIT\n\nMIT license text\n\n---\n\n### Apache-2.0\n\nApache license text"
        );

        assert_eq!(entries[1].name, "no-repo-crate");
        assert_eq!(entries[1].homepage, None);
        assert_eq!(entries[1].text, "### MIT\n\nMIT license text");
    }
}
