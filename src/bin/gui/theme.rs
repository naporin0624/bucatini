use eframe::egui;

/// Colors shared by more than one module. `install_style` below is the source of truth
/// for the cannelloni theme's full palette; only the subset other modules (`main.rs`
/// status line, `licenses.rs` viewer) also need is re-exported here, so those callers
/// can't drift out of sync with the palette `install_style` actually applies.
pub mod tokens {
    use eframe::egui::Color32;

    pub const SURFACE: Color32 = Color32::from_rgb(0x1C, 0x1C, 0x1C);
    pub const TEXT: Color32 = Color32::from_rgb(0xEE, 0xEF, 0xF2);
    pub const TEXT_MUTED: Color32 = Color32::from_rgb(0xC9, 0xCC, 0xD2);
    pub const TEXT_DIM: Color32 = Color32::from_rgb(0x9E, 0x9E, 0x9E);
    pub const ACCENT: Color32 = Color32::from_rgb(0x39, 0x96, 0xFF);
}

/// Install the bundled LINE Seed JP font and the cannelloni dark theme.
pub fn install_style(ctx: &egui::Context) {
    use std::sync::Arc;

    // --- Fonts: LINE Seed JP as the primary proportional face, keeping
    //     egui's default fonts (incl. emoji) as fallbacks. ---
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "LINESeedJP".to_owned(),
        Arc::new(egui::FontData::from_static(include_bytes!(
            "../../../vendor/fonts/LINESeedJP-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(egui::FontFamily::Proportional)
        .or_default()
        .insert(0, "LINESeedJP".to_owned());
    fonts
        .families
        .entry(egui::FontFamily::Monospace)
        .or_default()
        .push("LINESeedJP".to_owned());
    ctx.set_fonts(fonts);

    // --- Theme: cannelloni (dark, neo-brutalist: 0 corners, visible borders, no shadow). ---
    use egui::{Color32, CornerRadius, Stroke};
    let canvas = Color32::from_rgb(0x12, 0x12, 0x12);
    let surface = tokens::SURFACE;
    let muted = Color32::from_rgb(0x26, 0x26, 0x26);
    let emphasis = Color32::from_rgb(0x2E, 0x2E, 0x2E);
    let text = tokens::TEXT;
    let text_muted = tokens::TEXT_MUTED;
    let border = Color32::from_rgb(0x69, 0x69, 0x69);
    let border_subtle = Color32::from_rgb(0x69, 0x69, 0x69);
    let border_strong = Color32::from_rgb(0x86, 0x86, 0x86);
    let accent = tokens::ACCENT;

    let mut v = egui::Visuals::dark();
    v.panel_fill = canvas;
    v.window_fill = canvas;
    v.faint_bg_color = surface;
    v.extreme_bg_color = surface;
    v.window_shadow = egui::epaint::Shadow::NONE;
    v.popup_shadow = egui::epaint::Shadow::NONE;
    v.window_corner_radius = CornerRadius::ZERO;
    v.menu_corner_radius = CornerRadius::ZERO;
    v.hyperlink_color = accent;
    v.selection.bg_fill = accent.gamma_multiply(0.45);
    v.selection.stroke = Stroke::new(1.0, accent);

    let w = &mut v.widgets;
    // noninteractive: labels / panel
    w.noninteractive.bg_fill = canvas;
    w.noninteractive.weak_bg_fill = canvas;
    w.noninteractive.bg_stroke = Stroke::new(1.0, border_subtle);
    w.noninteractive.fg_stroke = Stroke::new(1.0, text_muted);
    w.noninteractive.corner_radius = CornerRadius::ZERO;
    // inactive: idle buttons / combo
    w.inactive.bg_fill = surface;
    w.inactive.weak_bg_fill = surface;
    w.inactive.bg_stroke = Stroke::new(1.0, border);
    w.inactive.fg_stroke = Stroke::new(1.0, text);
    w.inactive.corner_radius = CornerRadius::ZERO;
    // hovered
    w.hovered.bg_fill = muted;
    w.hovered.weak_bg_fill = muted;
    w.hovered.bg_stroke = Stroke::new(1.0, border_strong);
    w.hovered.fg_stroke = Stroke::new(1.0, text);
    w.hovered.corner_radius = CornerRadius::ZERO;
    // active (pressed) + open (combo open)
    for wv in [&mut w.active, &mut w.open] {
        wv.bg_fill = emphasis;
        wv.weak_bg_fill = emphasis;
        wv.bg_stroke = Stroke::new(1.0, accent);
        wv.fg_stroke = Stroke::new(1.0, text);
        wv.corner_radius = CornerRadius::ZERO;
    }

    ctx.set_visuals(v);
    ctx.global_style_mut(|s| s.spacing.item_spacing = egui::vec2(8.0, 8.0));
}
