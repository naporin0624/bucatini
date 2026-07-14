//! Native application menu (macOS: `bucatini ▸ About / Quit`, `Help ▸ 使い方…`)
//! built with `muda`.
//!
//! `muda` is a normal (not platform-gated) dependency, so this module compiles on
//! every target; only the `Menu::init_for_nsapp` call inside `build_app_menu` —
//! which installs the menu as the app's NSApp main menu — is macOS-only.

use muda::accelerator::{Accelerator, Code, Modifiers};
use muda::{AboutMetadata, Menu, MenuId, MenuItem, PredefinedMenuItem, Submenu};

/// What the running app should do in response to a menu activation. `About` is
/// a native `PredefinedMenuItem` handled entirely by the OS — it never surfaces
/// here. `Quit` is a custom item (see `build_app_menu`'s doc comment for why
/// `PredefinedMenuItem::quit` isn't used) and does surface here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenuAction {
    OpenUsage,
    OpenLicenses,
    Quit,
}

/// Assembles this app's `AboutMetadata`. Kept as a pure function (no globals, no
/// I/O) so its contents are unit-testable without constructing a real menu, and
/// so the tray menu's `About Bucatini…` item can reuse the exact same metadata.
pub fn about_metadata() -> AboutMetadata {
    AboutMetadata {
        name: Some("Bucatini".to_string()),
        version: Some(env!("CARGO_PKG_VERSION").to_string()),
        authors: Some(vec!["naporitan".to_string()]),
        copyright: Some("\u{a9} 2026 naporitan".to_string()),
        website: Some("https://napochaan.com".to_string()),
        ..Default::default()
    }
}

/// Maps a fired `MenuEvent`'s id to the `MenuAction` it represents. `None`
/// covers ids muda already handled natively (About) or any id from a menu we
/// didn't build.
fn action_for(
    event_id: &MenuId,
    usage_id: &MenuId,
    licenses_id: &MenuId,
    quit_id: &MenuId,
) -> Option<MenuAction> {
    if event_id == usage_id {
        Some(MenuAction::OpenUsage)
    } else if event_id == licenses_id {
        Some(MenuAction::OpenLicenses)
    } else if event_id == quit_id {
        Some(MenuAction::Quit)
    } else {
        None
    }
}

/// The app's native menu bar, plus the id needed to recognize its one custom item.
pub struct AppMenu {
    // Never read again after `build_app_menu` installs it, but must stay alive
    // for the app's lifetime: dropping `Menu` frees its native (NSMenu on macOS)
    // backing storage, which would tear down the menu bar it was just installed
    // as. The field is write-only in every build, so it needs an unconditional
    // `#[allow(dead_code)]`.
    #[allow(dead_code)]
    menu: Menu,
    usage_id: MenuId,
    licenses_id: MenuId,
    quit_id: MenuId,
}

impl AppMenu {
    /// Maps a fired `MenuEvent`'s id to this menu's `MenuAction`, if any.
    pub fn action_for(&self, id: &MenuId) -> Option<MenuAction> {
        action_for(id, &self.usage_id, &self.licenses_id, &self.quit_id)
    }
}

/// Builds the `bucatini ▸ About / Quit` and `Help ▸ 使い方…` menu.
///
/// On macOS this also installs it as the app's main menu (`init_for_nsapp`) —
/// safe to call here because `build_app_menu` is only ever invoked from
/// `GuiApp::new`, which eframe calls after `NSApplication` already exists.
pub fn build_app_menu() -> Result<AppMenu, muda::Error> {
    let usage_id = MenuId::new("bucatini-usage");
    let usage_item = MenuItem::with_id(
        usage_id.clone(),
        "\u{4F7F}\u{3044}\u{65B9}\u{2026}",
        true,
        None,
    );

    // Not `PredefinedMenuItem::quit`: its native action is `[NSApp terminate:]`,
    // which bypasses `MenuEvent` entirely and exits before the NDI/Syphon worker
    // thread can be signaled and joined, so the worker's Drop cleanup (e.g.
    // un-publishing the Syphon server) never runs. A custom item routes through
    // `action_for` → `MenuAction::Quit` → `GuiApp::quit`, same as the tray's
    // quit item, so shutdown always goes through the worker join.
    let quit_id = MenuId::new("bucatini-quit");
    let quit_accelerator = Accelerator::new(Some(Modifiers::META), Code::KeyQ);
    let quit_item = MenuItem::with_id(
        quit_id.clone(),
        "Quit bucatini",
        true,
        Some(quit_accelerator),
    );

    let licenses_id = MenuId::new("bucatini-licenses");
    let licenses_item = MenuItem::with_id(
        licenses_id.clone(),
        "Open Source Licenses\u{2026}",
        true,
        None,
    );

    let app_submenu = Submenu::with_items(
        "bucatini",
        true,
        &[
            &PredefinedMenuItem::about(None, Some(about_metadata())),
            &PredefinedMenuItem::separator(),
            &quit_item,
        ],
    )?;
    let help_submenu = Submenu::with_items("Help", true, &[&usage_item, &licenses_item])?;

    let menu = Menu::with_items(&[&app_submenu, &help_submenu])?;

    #[cfg(target_os = "macos")]
    menu.init_for_nsapp();

    Ok(AppMenu {
        menu,
        usage_id,
        licenses_id,
        quit_id,
    })
}

#[cfg(test)]
mod tests {
    use super::{about_metadata, action_for, MenuAction};
    use muda::MenuId;

    #[test]
    fn about_metadata_has_the_expected_fields() {
        let metadata = about_metadata();

        assert_eq!(metadata.name, Some("Bucatini".to_string()));
        assert_eq!(
            metadata.version,
            Some(env!("CARGO_PKG_VERSION").to_string())
        );
        assert_eq!(metadata.authors, Some(vec!["naporitan".to_string()]));
        assert_eq!(
            metadata.copyright,
            Some("\u{a9} 2026 naporitan".to_string())
        );
        assert_eq!(metadata.website, Some("https://napochaan.com".to_string()));
    }

    #[test]
    fn action_for_maps_the_usage_id_to_open_usage() {
        let usage_id = MenuId::new("bucatini-usage");
        let licenses_id = MenuId::new("bucatini-licenses");
        let quit_id = MenuId::new("bucatini-quit");

        assert_eq!(
            action_for(&usage_id, &usage_id, &licenses_id, &quit_id),
            Some(MenuAction::OpenUsage)
        );
    }

    #[test]
    fn action_for_maps_the_licenses_id_to_open_licenses() {
        let usage_id = MenuId::new("bucatini-usage");
        let licenses_id = MenuId::new("bucatini-licenses");
        let quit_id = MenuId::new("bucatini-quit");

        assert_eq!(
            action_for(&licenses_id, &usage_id, &licenses_id, &quit_id),
            Some(MenuAction::OpenLicenses)
        );
    }

    #[test]
    fn action_for_maps_the_quit_id_to_quit() {
        let usage_id = MenuId::new("bucatini-usage");
        let licenses_id = MenuId::new("bucatini-licenses");
        let quit_id = MenuId::new("bucatini-quit");

        assert_eq!(
            action_for(&quit_id, &usage_id, &licenses_id, &quit_id),
            Some(MenuAction::Quit)
        );
    }

    #[test]
    fn action_for_ignores_ids_it_does_not_recognize() {
        let usage_id = MenuId::new("bucatini-usage");
        let licenses_id = MenuId::new("bucatini-licenses");
        let quit_id = MenuId::new("bucatini-quit");
        let other_id = MenuId::new("some-other-item");

        assert_eq!(
            action_for(&other_id, &usage_id, &licenses_id, &quit_id),
            None
        );
    }
}
