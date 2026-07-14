use tray_icon::menu::{Menu, MenuId, MenuItem, PredefinedMenuItem};
use tray_icon::{TrayIcon, TrayIconBuilder};

/// Owns the tray icon + the handles needed to update/route its menu.
pub struct TrayState {
    _icon: TrayIcon,
    pub status: MenuItem,
    pub quit_id: MenuId,
}

/// A tiny solid accent-blue square icon (32x32 RGBA), generated in code.
fn tray_icon_image() -> tray_icon::Icon {
    const N: u32 = 32;
    let mut rgba = Vec::with_capacity((N * N * 4) as usize);
    for _ in 0..(N * N) {
        rgba.extend_from_slice(&[0x39, 0x96, 0xFF, 0xFF]);
    }
    tray_icon::Icon::from_rgba(rgba, N, N).expect("valid 32x32 RGBA icon")
}

/// Build the tray icon with a status line and Quit. Returns None on failure
/// (the app still runs windowed without a tray).
pub fn build_tray() -> Option<TrayState> {
    let menu = Menu::new();
    let status = MenuItem::new("Idle", true, None);
    let quit = MenuItem::new("Quit", true, None);
    menu.append(&status).ok()?;
    menu.append(&PredefinedMenuItem::separator()).ok()?;
    menu.append(&quit).ok()?;
    let icon = TrayIconBuilder::new()
        .with_tooltip("Bucatini")
        .with_icon(tray_icon_image())
        .with_menu(Box::new(menu))
        .build()
        .ok()?;
    Some(TrayState {
        _icon: icon,
        status,
        quit_id: quit.id().clone(),
    })
}
