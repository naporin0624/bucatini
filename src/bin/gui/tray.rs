//! Menu-bar tray icon (status line / About / 使い方 / Quit) built with `tray-icon`.
//!
//! `tray-icon` re-exports the same `muda` version this crate depends on directly
//! (verified via `cargo tree --features gui -i muda`), so `muda::MenuEvent` and
//! `tray_icon::menu::MenuEvent` are the same type on the same global event
//! channel — no separate tray-event plumbing is needed, only `MenuId` mapping.

use muda::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{Icon, TrayIcon, TrayIconBuilder};

use crate::menu::about_metadata;

const TRAY_ICON_BYTES: &[u8] = include_bytes!("../../../assets/icon/tray-44.png");

/// What the running app should do in response to a tray menu activation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrayAction {
    Show,
    OpenUsage,
    Quit,
}

/// A decoded RGBA image ready for `tray_icon::Icon::from_rgba`.
struct DecodedIcon {
    rgba: Vec<u8>,
    width: u32,
    height: u32,
}

#[derive(Debug, thiserror::Error)]
pub enum TrayError {
    #[error(transparent)]
    Png(#[from] png::DecodingError),
    #[error(transparent)]
    BadIcon(#[from] tray_icon::BadIcon),
    #[error(transparent)]
    Tray(#[from] tray_icon::Error),
    #[error(transparent)]
    Menu(#[from] muda::Error),
    #[error("tray icon PNG is not 8-bit RGB or RGBA")]
    UnexpectedFormat,
}

/// Decodes `bytes` as a PNG into raw RGBA. Accepts both 8-bit RGB (the format
/// the committed asset ships in — no alpha channel is worth the extra bytes for
/// a solid glyph) and 8-bit RGBA, expanding RGB to RGBA by appending an opaque
/// alpha byte per pixel; any other bit depth or color type is rejected.
fn decode_icon(bytes: &[u8]) -> Result<DecodedIcon, TrayError> {
    let mut reader = png::Decoder::new(std::io::Cursor::new(bytes)).read_info()?;
    // `output_buffer_size` returns `None` only when the frame doesn't fit this
    // machine's address space; `unwrap_or(0)` lets that case fall through to
    // `next_frame`'s own buffer-too-small `DecodingError` rather than needing a
    // second error path here.
    let mut buffer = vec![0u8; reader.output_buffer_size().unwrap_or(0)];
    let info = reader.next_frame(&mut buffer)?;
    buffer.truncate(info.buffer_size());

    if info.bit_depth != png::BitDepth::Eight {
        return Err(TrayError::UnexpectedFormat);
    }

    let rgba = match info.color_type {
        png::ColorType::Rgba => buffer,
        png::ColorType::Rgb => buffer
            .chunks_exact(3)
            .flat_map(|rgb| [rgb[0], rgb[1], rgb[2], 0xFF])
            .collect(),
        _ => return Err(TrayError::UnexpectedFormat),
    };

    Ok(DecodedIcon {
        rgba,
        width: info.width,
        height: info.height,
    })
}

/// Maps a fired `MenuEvent`'s id to the `TrayAction` it represents. `None`
/// covers any id from a menu we didn't build (e.g. the app menu's usage item,
/// or a foreign event).
fn action_for(
    id: &MenuId,
    show_id: &MenuId,
    usage_id: &MenuId,
    quit_id: &MenuId,
) -> Option<TrayAction> {
    if id == show_id {
        Some(TrayAction::Show)
    } else if id == usage_id {
        Some(TrayAction::OpenUsage)
    } else if id == quit_id {
        Some(TrayAction::Quit)
    } else {
        None
    }
}

/// The tray icon, plus the handles needed to update/route its menu.
pub struct AppTray {
    // Never read again after `build_tray` installs it, but must stay alive for
    // the app's lifetime: dropping `TrayIcon` removes the status item from the
    // menu bar. Write-only in every build, so it needs an unconditional
    // `#[allow(dead_code)]` (mirrors `AppMenu::menu` in menu.rs).
    #[allow(dead_code)]
    tray: TrayIcon,
    pub status: MenuItem,
    usage_id: MenuId,
    quit_id: MenuId,
}

impl AppTray {
    /// Maps a fired `MenuEvent`'s id to this tray's `TrayAction`, if any.
    pub fn action_for(&self, id: &MenuId) -> Option<TrayAction> {
        action_for(id, self.status.id(), &self.usage_id, &self.quit_id)
    }
}

/// Builds the menu-bar tray icon with a status line, `About Bucatini…`,
/// `使い方…`, and `Quit`. Clicking the status line shows the window (routed by
/// the caller via `action_for`); its text is updated live via `AppTray::status`.
pub fn build_tray() -> Result<AppTray, TrayError> {
    let usage_id = MenuId::new("bucatini-tray-usage");
    let quit_id = MenuId::new("bucatini-tray-quit");

    let status = MenuItem::new("Idle", true, None);
    let usage_item = MenuItem::with_id(
        usage_id.clone(),
        "\u{4F7F}\u{3044}\u{65B9}\u{2026}",
        true,
        None,
    );
    let quit_item = MenuItem::with_id(quit_id.clone(), "Quit", true, None);

    let menu = Menu::with_items(&[
        &status,
        &PredefinedMenuItem::separator(),
        &PredefinedMenuItem::about(Some("About Bucatini\u{2026}"), Some(about_metadata())),
        &usage_item,
        &PredefinedMenuItem::separator(),
        &quit_item,
    ])?;

    let decoded = decode_icon(TRAY_ICON_BYTES)?;
    let icon = Icon::from_rgba(decoded.rgba, decoded.width, decoded.height)?;

    let tray = TrayIconBuilder::new()
        .with_icon(icon)
        .with_tooltip("Bucatini")
        .with_menu(Box::new(menu))
        .build()?;

    Ok(AppTray {
        tray,
        status,
        usage_id,
        quit_id,
    })
}

#[cfg(test)]
mod tests {
    use muda::MenuId;

    use super::{action_for, decode_icon, TrayAction, TRAY_ICON_BYTES};

    #[test]
    fn decode_icon_reads_the_embedded_asset_as_44x44_rgba() {
        let decoded = decode_icon(TRAY_ICON_BYTES).unwrap();

        assert_eq!(decoded.width, 44);
        assert_eq!(decoded.height, 44);
        assert_eq!(decoded.rgba.len(), 44 * 44 * 4);
    }

    #[test]
    fn action_for_maps_the_show_id_to_show() {
        let show_id = MenuId::new("bucatini-tray-show");
        let usage_id = MenuId::new("bucatini-tray-usage");
        let quit_id = MenuId::new("bucatini-tray-quit");

        assert_eq!(
            action_for(&show_id, &show_id, &usage_id, &quit_id),
            Some(TrayAction::Show)
        );
    }

    #[test]
    fn action_for_maps_the_usage_id_to_open_usage() {
        let show_id = MenuId::new("bucatini-tray-show");
        let usage_id = MenuId::new("bucatini-tray-usage");
        let quit_id = MenuId::new("bucatini-tray-quit");

        assert_eq!(
            action_for(&usage_id, &show_id, &usage_id, &quit_id),
            Some(TrayAction::OpenUsage)
        );
    }

    #[test]
    fn action_for_maps_the_quit_id_to_quit() {
        let show_id = MenuId::new("bucatini-tray-show");
        let usage_id = MenuId::new("bucatini-tray-usage");
        let quit_id = MenuId::new("bucatini-tray-quit");

        assert_eq!(
            action_for(&quit_id, &show_id, &usage_id, &quit_id),
            Some(TrayAction::Quit)
        );
    }

    #[test]
    fn action_for_ignores_ids_it_does_not_recognize() {
        let show_id = MenuId::new("bucatini-tray-show");
        let usage_id = MenuId::new("bucatini-tray-usage");
        let quit_id = MenuId::new("bucatini-tray-quit");
        let other_id = MenuId::new("some-other-item");

        assert_eq!(action_for(&other_id, &show_id, &usage_id, &quit_id), None);
    }
}
