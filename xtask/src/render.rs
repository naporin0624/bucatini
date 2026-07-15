use crate::license_entry::LicenseEntry;

const SEPARATOR: &str =
    "================================================================================";

pub fn render_notices(entries: &[LicenseEntry]) -> String {
    let mut out = String::new();
    out.push_str("THIRD-PARTY SOFTWARE NOTICES AND INFORMATION\n\n");
    out.push_str("This package incorporates components from the projects listed below.\n");

    for entry in entries {
        out.push('\n');
        out.push_str(SEPARATOR);
        out.push_str("\n\n");
        match &entry.version {
            Some(version) => out.push_str(&format!("{} {}\n", entry.name, version)),
            None => out.push_str(&format!("{}\n", entry.name)),
        }
        out.push_str(&entry.license);
        out.push('\n');
        if let Some(homepage) = &entry.homepage {
            out.push_str(homepage);
            out.push('\n');
        }
        out.push('\n');
        out.push_str(&entry.text);
        out.push('\n');
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::license_entry::LicenseCategory;

    #[test]
    fn renders_header_and_entries_with_separators() {
        let entries = vec![
            LicenseEntry {
                name: "libfoo".to_string(),
                version: Some("1.0.0".to_string()),
                license: "MIT".to_string(),
                text: "MIT TEXT".to_string(),
                homepage: Some("https://example.com/libfoo".to_string()),
                category: LicenseCategory::Library,
            },
            LicenseEntry {
                name: "Syphon Framework".to_string(),
                version: None,
                license: "BSD-3-Clause".to_string(),
                text: "BSD TEXT".to_string(),
                homepage: Some("https://github.com/Syphon/Syphon-Framework".to_string()),
                category: LicenseCategory::Native,
            },
        ];

        let rendered = render_notices(&entries);

        let expected = "THIRD-PARTY SOFTWARE NOTICES AND INFORMATION\n\nThis package incorporates components from the projects listed below.\n\n================================================================================\n\nlibfoo 1.0.0\nMIT\nhttps://example.com/libfoo\n\nMIT TEXT\n\n================================================================================\n\nSyphon Framework\nBSD-3-Clause\nhttps://github.com/Syphon/Syphon-Framework\n\nBSD TEXT\n";

        assert_eq!(rendered, expected);
    }

    #[test]
    fn entry_without_homepage_omits_the_homepage_line() {
        let entries = vec![LicenseEntry {
            name: "no-homepage-crate".to_string(),
            version: Some("2.0.0".to_string()),
            license: "MIT".to_string(),
            text: "TEXT".to_string(),
            homepage: None,
            category: LicenseCategory::Library,
        }];

        let rendered = render_notices(&entries);

        assert!(rendered.contains("no-homepage-crate 2.0.0\nMIT\n\nTEXT\n"));
    }

    #[test]
    fn rendering_the_same_entries_twice_produces_identical_output() {
        let entries = vec![LicenseEntry {
            name: "libfoo".to_string(),
            version: Some("1.0.0".to_string()),
            license: "MIT".to_string(),
            text: "TEXT".to_string(),
            homepage: None,
            category: LicenseCategory::Library,
        }];

        assert_eq!(render_notices(&entries), render_notices(&entries));
    }
}
