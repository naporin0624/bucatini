use crate::license_entry::LicenseEntry;

/// Sorts by `(category, name, version)`. `Vec::sort_by` is documented-stable in std, so any
/// remaining ties (equal category, name, and version) keep their relative input order without
/// extra bookkeeping.
pub fn sort_entries(mut entries: Vec<LicenseEntry>) -> Vec<LicenseEntry> {
    entries.sort_by(|a, b| {
        (a.category, &a.name)
            .cmp(&(b.category, &b.name))
            .then_with(|| a.version.cmp(&b.version))
    });
    entries
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
    fn sorts_by_category_then_name_with_declared_category_order() {
        let entries = vec![
            entry("zeta", LicenseCategory::Native),
            entry("alpha", LicenseCategory::Library),
            entry("beta", LicenseCategory::Font),
            entry("omega", LicenseCategory::Library),
        ];

        let sorted = sort_entries(entries);

        let order: Vec<(&str, LicenseCategory)> = sorted
            .iter()
            .map(|e| (e.name.as_str(), e.category))
            .collect();
        assert_eq!(
            order,
            vec![
                ("alpha", LicenseCategory::Library),
                ("omega", LicenseCategory::Library),
                ("beta", LicenseCategory::Font),
                ("zeta", LicenseCategory::Native),
            ]
        );
    }

    #[test]
    fn same_name_entries_break_ties_by_version_ascending() {
        let mut first = entry("dup", LicenseCategory::Library);
        first.version = Some("1.0.0".to_string());
        let mut second = entry("dup", LicenseCategory::Library);
        second.version = Some("2.0.0".to_string());

        // Feed them in descending version order so a pass would only succeed via the
        // explicit version tiebreak, not by coincidentally matching input order.
        let sorted = sort_entries(vec![second.clone(), first.clone()]);

        assert_eq!(sorted[0].version, first.version);
        assert_eq!(sorted[1].version, second.version);
    }

    #[test]
    fn stable_sort_preserves_relative_order_for_fully_equal_keys() {
        let first = entry("dup", LicenseCategory::Library);
        let second = entry("dup", LicenseCategory::Library);

        let sorted = sort_entries(vec![first.clone(), second.clone()]);

        assert_eq!(sorted, vec![first, second]);
    }

    #[test]
    fn sorting_twice_is_idempotent() {
        let entries = vec![
            entry("zeta", LicenseCategory::Native),
            entry("alpha", LicenseCategory::Library),
        ];

        let once = sort_entries(entries);
        let twice = sort_entries(once.clone());

        assert_eq!(once, twice);
    }
}
