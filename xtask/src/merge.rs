use std::collections::HashSet;

use crate::license_entry::LicenseEntry;

/// Merges scanner (`cargo bundle-licenses`) output with the hand-written appendix. On a name
/// collision the appendix entry wins and the scanner's is dropped — the appendix exists
/// specifically to override/supply entries the crate scanner cannot know about (Syphon
/// Framework, Spout2 SDK, NDI, LINE Seed JP), so it must never be shadowed by a same-named
/// crate dependency.
pub fn merge(scanner: Vec<LicenseEntry>, appendix: Vec<LicenseEntry>) -> Vec<LicenseEntry> {
    let appendix_names: HashSet<&str> = appendix.iter().map(|entry| entry.name.as_str()).collect();
    let mut merged: Vec<LicenseEntry> = scanner
        .into_iter()
        .filter(|entry| !appendix_names.contains(entry.name.as_str()))
        .collect();
    merged.extend(appendix);
    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_entry::LicenseCategory;

    fn entry(name: &str, category: LicenseCategory) -> LicenseEntry {
        LicenseEntry {
            name: name.to_string(),
            version: None,
            license: "MIT".to_string(),
            text: "text".to_string(),
            homepage: None,
            category,
        }
    }

    #[test]
    fn appendix_entry_replaces_scanner_entry_with_same_name() {
        let scanner = vec![
            entry("eframe", LicenseCategory::Library),
            entry("Syphon Framework", LicenseCategory::Library),
        ];
        let appendix = vec![entry("Syphon Framework", LicenseCategory::Native)];

        let merged = merge(scanner, appendix);

        assert_eq!(merged.len(), 2);
        let syphon = merged
            .iter()
            .find(|e| e.name == "Syphon Framework")
            .unwrap();
        assert_eq!(syphon.category, LicenseCategory::Native);
        assert!(merged.iter().any(|e| e.name == "eframe"));
    }

    #[test]
    fn no_collision_keeps_every_entry() {
        let scanner = vec![entry("eframe", LicenseCategory::Library)];
        let appendix = vec![entry("Syphon Framework", LicenseCategory::Native)];

        let merged = merge(scanner, appendix);

        assert_eq!(merged.len(), 2);
    }

    #[test]
    fn all_appendix_entries_survive_a_merge_against_empty_scanner_output() {
        let appendix = vec![
            entry("Syphon Framework", LicenseCategory::Native),
            entry("Spout2 SDK", LicenseCategory::Native),
            entry("NDI", LicenseCategory::Native),
            entry("LINE Seed JP", LicenseCategory::Font),
        ];

        let merged = merge(Vec::new(), appendix.clone());

        assert_eq!(merged, appendix);
    }
}
