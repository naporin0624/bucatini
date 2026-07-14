//! "Open Source Licenses" window — search/filter viewer over the generated
//! `assets/third-party-licenses.json` manifest (see `cargo xtask gen-licenses`).

use std::collections::HashSet;
use std::sync::OnceLock;

use eframe::egui;

/// Which of the three sources a license entry came from. `Font`/`Native` entries come
/// from the hand-written appendix (Syphon Framework, LINE Seed JP, \u{2026}) that
/// `cargo xtask gen-licenses` merges in; `Library` is every Rust crate dependency
/// `cargo-bundle-licenses` discovers.
#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LicenseCategory {
    Library,
    Font,
    Native,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Deserialize)]
pub struct LicenseEntry {
    pub name: String,
    pub version: Option<String>,
    pub license: String,
    pub text: String,
    pub homepage: Option<String>,
    pub category: LicenseCategory,
}

/// The generated+committed license manifest written by `cargo xtask gen-licenses`.
/// `include_str!` makes a missing/unreadable file a *compile* error \u{2014} the only failure mode
/// left at runtime is malformed *content*, which `parse_licenses`'s `Result` surfaces instead of
/// panicking.
const EMBEDDED_LICENSES_JSON: &str = include_str!("../../../assets/third-party-licenses.json");

/// Parses the embedded manifest. Never panics.
pub fn parse_licenses(json: &str) -> Result<Vec<LicenseEntry>, serde_json::Error> {
    serde_json::from_str(json)
}

/// Case-insensitive substring match on `name`, AND'd with an optional exact `category` match.
/// `query = ""` matches every entry (the empty string is a substring of everything) \u{2014} this is
/// the window's initial "nothing typed yet" state.
pub fn filter_entries<'a>(
    entries: &'a [LicenseEntry],
    query: &str,
    category: Option<LicenseCategory>,
) -> Vec<&'a LicenseEntry> {
    let query_lower = query.to_lowercase();
    entries
        .iter()
        .filter(|entry| entry.name.to_lowercase().contains(&query_lower))
        .filter(|entry| {
            category
                .as_ref()
                .is_none_or(|wanted| &entry.category == wanted)
        })
        .collect()
}

/// Display text for a possibly-absent version \u{2014} font/native appendix entries have no crate
/// version to show.
fn version_display(version: &Option<String>) -> &str {
    version.as_deref().unwrap_or("\u{2014}")
}

/// Right-aligned license badge text. Crates commonly express dual licenses in SPDX `OR` form
/// (`"MIT OR Apache-2.0"`); normalized to the more compact `"MIT / Apache-2.0"` for the badge.
fn license_badge_text(license: &str) -> String {
    license.replace(" OR ", " / ")
}

// Local color tokens, mirroring the pattern already used by
// `GuiApp::draw_status` in main.rs (this crate has no shared `theme::tokens`
// module to draw from).
const TEXT_SUBTLE: egui::Color32 = egui::Color32::from_rgb(0x9E, 0x9E, 0x9E);
const TEXT_MUTED: egui::Color32 = egui::Color32::from_rgb(0xC9, 0xCC, 0xD2);
const BG_MUTED: egui::Color32 = egui::Color32::from_rgb(0x1C, 0x1C, 0x1C);
const DANGER: egui::Color32 = egui::Color32::from_rgb(0xFF, 0x6B, 0x6B);

fn category_toggle(
    ui: &mut egui::Ui,
    current: &mut Option<LicenseCategory>,
    value: Option<LicenseCategory>,
    label: &str,
) {
    if ui.selectable_label(*current == value, label).clicked() {
        *current = value;
    }
}

/// Draws one row per filtered entry. `all_entries` (not `filtered`) is what `expanded`'s indices
/// are keyed against: `filtered` is rebuilt fresh from `filter_entries` every frame, so a row's
/// position within it shifts as the search box/category filter change \u{2014} keying `expanded` off
/// `filtered`'s own position would silently reattach the "expanded" flag to a different entry
/// the moment the visible set changes shape. Keying off `all_entries`'s stable index avoids
/// that; `filtered`'s items are `&LicenseEntry` borrowed from `all_entries`, so `std::ptr::eq`
/// recovers the original index without re-running the filter's own matching logic.
fn render_entry_list(
    ui: &mut egui::Ui,
    all_entries: &[LicenseEntry],
    filtered: &[&LicenseEntry],
    expanded: &mut HashSet<usize>,
) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        for entry in filtered {
            let Some(index) = all_entries
                .iter()
                .position(|candidate| std::ptr::eq(candidate, *entry))
            else {
                continue;
            };

            // `ui.horizontal`'s own `Response` only senses hover, so `.interact` is called to
            // add click sensing to the whole row's rect \u{2014} clicking anywhere in the row
            // (name, version, or badge) toggles expansion, not just the name text.
            let row_response = ui
                .horizontal(|ui| {
                    ui.label(&entry.name);
                    ui.label(version_display(&entry.version));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.colored_label(TEXT_SUBTLE, license_badge_text(&entry.license));
                    });
                })
                .response
                .interact(egui::Sense::click());

            if row_response.clicked() {
                if expanded.contains(&index) {
                    expanded.remove(&index);
                } else {
                    expanded.insert(index);
                }
            }

            if expanded.contains(&index) {
                egui::Frame::NONE
                    .fill(BG_MUTED)
                    .inner_margin(8.0)
                    .show(ui, |ui| {
                        ui.colored_label(TEXT_MUTED, &entry.text);
                        if let Some(homepage) = &entry.homepage {
                            ui.hyperlink_to(homepage, homepage);
                        }
                    });
            }

            ui.add(egui::Separator::default().spacing(0.0));
        }
    });
}

fn viewport_id() -> egui::ViewportId {
    egui::ViewportId::from_hash_of("bucatini-licenses-window")
}

/// Uses `show_viewport_immediate` rather than a deferred viewport: a deferred viewport's callback
/// must be `Send + Sync + 'static`, so it cannot borrow `&mut self`'s fields directly and would
/// need `self.query`/`self.category`/`self.expanded`/`self.open` wrapped in `Arc<Mutex<_>>` just
/// to reach them. The immediate callback runs inline on the calling thread and can borrow those
/// fields as plain `&mut` references instead. This window draws no continuously-animating content
/// that would need the deferred variant's independent repaint scheduling, so there is no
/// offsetting benefit to give up in exchange.
#[derive(Default)]
pub struct LicensesWindow {
    open: bool,
    query: String,
    category: Option<LicenseCategory>,
    expanded: HashSet<usize>,
    data: OnceLock<Result<Vec<LicenseEntry>, serde_json::Error>>,
}

impl LicensesWindow {
    /// Opens the window, or focuses it if it's already open (mirrors `UsageWindow::request_open`).
    pub fn request_open(&mut self, ctx: &egui::Context) {
        if self.open {
            ctx.send_viewport_cmd_to(viewport_id(), egui::ViewportCommand::Focus);
        } else {
            self.open = true;
        }
    }

    /// Renders the licenses viewport if open; a no-op otherwise. Call unconditionally every
    /// frame \u{2014} it self-gates on `open` and does nothing when closed.
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.open {
            return;
        }

        let entries_result = self
            .data
            .get_or_init(|| parse_licenses(EMBEDDED_LICENSES_JSON));
        let query = &mut self.query;
        let category = &mut self.category;
        let expanded = &mut self.expanded;
        let mut still_open = true;

        ctx.show_viewport_immediate(
            viewport_id(),
            egui::ViewportBuilder::default()
                .with_title("Open Source Licenses \u{2014} Bucatini")
                .with_inner_size([640.0, 520.0]),
            |ui, _class| {
                ui.horizontal(|ui| {
                    ui.text_edit_singleline(query);
                    ui.add(egui::Separator::default().vertical());
                    category_toggle(ui, category, None, "All");
                    category_toggle(ui, category, Some(LicenseCategory::Library), "Library");
                    category_toggle(ui, category, Some(LicenseCategory::Font), "Font");
                    category_toggle(ui, category, Some(LicenseCategory::Native), "Native");
                });
                ui.separator();

                match entries_result {
                    Ok(entries) => {
                        let filtered = filter_entries(entries, query, category.clone());
                        render_entry_list(ui, entries, &filtered, expanded);
                    }
                    Err(error) => {
                        ui.colored_label(
                            DANGER,
                            format!("failed to load bundled licenses: {error}"),
                        );
                    }
                }

                if ui.ctx().input(|i| i.viewport().close_requested()) {
                    still_open = false;
                }
            },
        );
        self.open = still_open;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE_JSON: &str = r#"[
        {
            "name": "serde",
            "version": "1.0.210",
            "license": "MIT OR Apache-2.0",
            "text": "MIT License text goes here.",
            "homepage": "https://serde.rs",
            "category": "library"
        },
        {
            "name": "LINE Seed JP",
            "version": null,
            "license": "OFL-1.1",
            "text": "SIL Open Font License text goes here.",
            "homepage": null,
            "category": "font"
        }
    ]"#;

    #[test]
    fn parse_licenses_reads_a_well_formed_manifest() {
        let entries = parse_licenses(SAMPLE_JSON).unwrap();

        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].name, "serde");
        assert_eq!(entries[0].version, Some("1.0.210".to_string()));
        assert_eq!(entries[0].category, LicenseCategory::Library);
        assert_eq!(entries[1].name, "LINE Seed JP");
        assert_eq!(entries[1].version, None);
        assert_eq!(entries[1].category, LicenseCategory::Font);
    }

    #[test]
    fn parse_licenses_reports_malformed_json_as_an_error_not_a_panic() {
        let result = parse_licenses("{ not valid json");

        assert!(result.is_err());
    }

    #[test]
    fn filter_entries_matches_name_case_insensitively() {
        let entries = parse_licenses(SAMPLE_JSON).unwrap();

        let filtered = filter_entries(&entries, "SERDE", None);

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "serde");
    }

    #[test]
    fn filter_entries_by_category_only() {
        let entries = parse_licenses(SAMPLE_JSON).unwrap();

        let filtered = filter_entries(&entries, "", Some(LicenseCategory::Font));

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].name, "LINE Seed JP");
    }

    #[test]
    fn filter_entries_combines_query_and_category_with_and() {
        let entries = parse_licenses(SAMPLE_JSON).unwrap();

        // Matches the query but not the category -> excluded.
        let filtered = filter_entries(&entries, "serde", Some(LicenseCategory::Font));

        assert!(filtered.is_empty());
    }

    #[test]
    fn filter_entries_with_empty_query_and_no_category_returns_everything() {
        let entries = parse_licenses(SAMPLE_JSON).unwrap();

        let filtered = filter_entries(&entries, "", None);

        assert_eq!(filtered.len(), entries.len());
    }

    #[test]
    fn committed_asset_parses_and_contains_the_expected_entries() {
        let entries = parse_licenses(EMBEDDED_LICENSES_JSON).unwrap();

        for name in [
            "Syphon Framework",
            "Spout2 SDK",
            "NDI",
            "LINE Seed JP",
            "eframe",
            "tray-icon",
        ] {
            assert!(
                entries.iter().any(|e| e.name == name),
                "committed manifest is missing {name:?}"
            );
        }
    }

    #[test]
    fn version_display_shows_an_em_dash_for_a_missing_version() {
        assert_eq!(version_display(&None), "\u{2014}");
    }

    #[test]
    fn version_display_shows_the_version_when_present() {
        assert_eq!(version_display(&Some("1.2.3".to_string())), "1.2.3");
    }

    #[test]
    fn license_badge_text_leaves_a_single_license_unchanged() {
        assert_eq!(license_badge_text("MIT"), "MIT");
    }

    #[test]
    fn license_badge_text_normalizes_an_or_expression_to_a_slash() {
        assert_eq!(license_badge_text("MIT OR Apache-2.0"), "MIT / Apache-2.0");
    }
}
