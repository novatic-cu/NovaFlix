//! Tokens de «Nova Glass» (specs/001-novaflix-v2/design-system.md §3). Ningún otro módulo
//! usa colores literales.

use egui::{
    Color32, CornerRadius, FontData, FontDefinitions, FontFamily, FontId, Stroke, TextStyle,
};
use novaflix_core::Theme;

/// Todos los tokens del diseño se mantienen aunque alguno aún no se use (design-system.md §3).
#[allow(dead_code)]
#[derive(Clone, Copy, Debug)]
pub struct Palette {
    pub dark: bool,
    pub bg_base: Color32,
    pub bg_surface: Color32,
    pub bg_raised: Color32,
    pub bg_overlay: Color32,
    pub glass_fill: Color32,
    pub glass_hover: Color32,
    pub glass_stroke: Color32,
    pub text: Color32,
    pub text_2: Color32,
    pub text_3: Color32,
    pub accent: Color32,
    pub accent_hover: Color32,
    pub accent_2: Color32,
    /// Relleno de botones primarios: más oscuro que `accent` para que el texto blanco cumpla AA.
    pub accent_fill: Color32,
    pub accent_fill_2: Color32,
    pub on_accent: Color32,
    pub success: Color32,
    pub warning: Color32,
    pub danger: Color32,
}

pub const fn rgb(hex: u32) -> Color32 {
    Color32::from_rgb((hex >> 16) as u8, (hex >> 8) as u8, hex as u8)
}

impl Palette {
    pub const DARK: Palette = Palette {
        dark: true,
        bg_base: rgb(0x07090F),
        bg_surface: rgb(0x0E121C),
        bg_raised: rgb(0x151A27),
        bg_overlay: rgb(0x1C2233),
        glass_fill: Color32::from_rgba_premultiplied(18, 18, 18, 18),
        glass_hover: Color32::from_rgba_premultiplied(28, 28, 28, 28),
        glass_stroke: Color32::from_rgba_premultiplied(31, 31, 31, 31),
        text: rgb(0xF5F7FB),
        text_2: rgb(0xA9B1C3),
        text_3: rgb(0x7C869B),
        accent: rgb(0x0A84FF),
        accent_hover: rgb(0x3D9BFF),
        accent_2: rgb(0x5E5CE6),
        accent_fill: rgb(0x0A70E0),
        accent_fill_2: rgb(0x5E5CE6),
        on_accent: rgb(0xFFFFFF),
        success: rgb(0x30D158),
        warning: rgb(0xFFD60A),
        danger: rgb(0xFF453A),
    };

    pub const LIGHT: Palette = Palette {
        dark: false,
        bg_base: rgb(0xF4F6FA),
        bg_surface: rgb(0xFFFFFF),
        bg_raised: rgb(0xFFFFFF),
        bg_overlay: rgb(0xFFFFFF),
        glass_fill: Color32::from_rgba_premultiplied(0, 0, 0, 10),
        glass_hover: Color32::from_rgba_premultiplied(0, 0, 0, 18),
        glass_stroke: Color32::from_rgba_premultiplied(0, 0, 0, 20),
        text: rgb(0x0B1220),
        text_2: rgb(0x475069),
        text_3: rgb(0x667189),
        accent: rgb(0x0A6CDB),
        accent_hover: rgb(0x0858B5),
        accent_2: rgb(0x4B49C8),
        accent_fill: rgb(0x0A6CDB),
        accent_fill_2: rgb(0x4B49C8),
        on_accent: rgb(0xFFFFFF),
        success: rgb(0x1F8F3F),
        warning: rgb(0xA67C00),
        danger: rgb(0xD1342B),
    };

    pub fn of(theme: Theme, system_dark: bool) -> Palette {
        match theme {
            Theme::Dark => Palette::DARK,
            Theme::Light => Palette::LIGHT,
            Theme::System if system_dark => Palette::DARK,
            Theme::System => Palette::LIGHT,
        }
    }
}

/// Radios concéntricos (design-system §3.6).
#[allow(dead_code)]
pub mod radius {
    pub const XS: u8 = 6;
    pub const SM: u8 = 10;
    pub const MD: u8 = 16;
    pub const LG: u8 = 22;
    pub const XL: u8 = 28;
    pub const PILL: u8 = 255;
}

#[allow(dead_code)]
pub mod space {
    pub const S4: f32 = 4.0;
    pub const S8: f32 = 8.0;
    pub const S12: f32 = 12.0;
    pub const S16: f32 = 16.0;
    pub const S24: f32 = 24.0;
    pub const S32: f32 = 32.0;
    pub const S48: f32 = 48.0;
}

/// Familias de texto con nombre (egui no tiene «pesos»: cada uno es una familia).
pub fn medium() -> FontFamily {
    FontFamily::Name("inter-medium".into())
}
pub fn semibold() -> FontFamily {
    FontFamily::Name("inter-semibold".into())
}

pub fn display() -> FontId {
    FontId::new(40.0, semibold())
}
pub fn title1() -> FontId {
    FontId::new(28.0, semibold())
}
pub fn title2() -> FontId {
    FontId::new(22.0, semibold())
}
pub fn headline() -> FontId {
    FontId::new(17.0, semibold())
}
pub fn body() -> FontId {
    FontId::new(15.0, FontFamily::Proportional)
}
pub fn callout() -> FontId {
    FontId::new(13.0, medium())
}
pub fn caption() -> FontId {
    FontId::new(11.5, medium())
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = FontDefinitions::default();
    let inter = [
        (
            "inter-regular",
            &include_bytes!("../assets/fonts/Inter-400.ttf")[..],
        ),
        (
            "inter-medium",
            &include_bytes!("../assets/fonts/Inter-500.ttf")[..],
        ),
        (
            "inter-semibold",
            &include_bytes!("../assets/fonts/Inter-600.ttf")[..],
        ),
    ];
    for (name, bytes) in inter {
        fonts.font_data.insert(
            name.to_string(),
            std::sync::Arc::new(FontData::from_static(bytes)),
        );
    }
    // Inter primero; las fuentes por defecto de egui quedan de reserva (símbolos, emoji).
    let fallback: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .cloned()
        .unwrap_or_default();
    let family = |first: &str| {
        let mut list = vec![first.to_string()];
        list.extend(fallback.iter().cloned());
        list
    };
    fonts
        .families
        .insert(FontFamily::Proportional, family("inter-regular"));
    fonts.families.insert(medium(), family("inter-medium"));
    fonts.families.insert(semibold(), family("inter-semibold"));
    egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
    // Los iconos de Phosphor se añaden solo a `Proportional`: hay que copiarlos a las familias
    // con nombre, o los botones (que usan Medium/SemiBold) mostrarían cuadrados.
    let phosphor: Vec<String> = fonts
        .families
        .get(&FontFamily::Proportional)
        .map(|list| {
            list.iter()
                .filter(|n| n.to_lowercase().contains("phosphor"))
                .cloned()
                .collect()
        })
        .unwrap_or_default();
    for family in [medium(), semibold()] {
        if let Some(list) = fonts.families.get_mut(&family) {
            list.extend(phosphor.iter().cloned());
        }
    }
    ctx.set_fonts(fonts);
}

pub fn apply(ctx: &egui::Context, p: &Palette, scale: f32) {
    ctx.set_pixels_per_point(scale);
    let which = if p.dark {
        egui::Theme::Dark
    } else {
        egui::Theme::Light
    };
    ctx.set_theme(which);
    let mut style = (*ctx.style_of(which)).clone();

    style.text_styles = [
        (TextStyle::Heading, title1()),
        (TextStyle::Body, body()),
        (TextStyle::Button, callout()),
        (TextStyle::Small, caption()),
        (
            TextStyle::Monospace,
            FontId::new(13.0, FontFamily::Monospace),
        ),
    ]
    .into();

    let v = &mut style.visuals;
    v.dark_mode = p.dark;
    v.override_text_color = Some(p.text);
    v.panel_fill = p.bg_base;
    v.window_fill = p.bg_overlay;
    v.extreme_bg_color = p.bg_raised;
    v.faint_bg_color = p.glass_fill;
    v.hyperlink_color = p.accent;
    v.selection.bg_fill = p.accent.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0, p.accent);
    v.window_stroke = Stroke::new(1.0, p.glass_stroke);
    v.window_corner_radius = CornerRadius::same(radius::LG);
    v.menu_corner_radius = CornerRadius::same(radius::MD);

    let widget = |bg: Color32, stroke: Color32| egui::style::WidgetVisuals {
        bg_fill: bg,
        weak_bg_fill: bg,
        bg_stroke: Stroke::new(1.0, stroke),
        corner_radius: CornerRadius::same(radius::SM),
        fg_stroke: Stroke::new(1.0, p.text),
        expansion: 0.0,
    };
    v.widgets.noninteractive = widget(p.bg_surface, p.glass_stroke);
    v.widgets.inactive = widget(p.glass_fill, Color32::TRANSPARENT);
    v.widgets.hovered = widget(p.glass_hover, p.glass_stroke);
    v.widgets.active = widget(p.glass_hover, p.accent);
    v.widgets.open = widget(p.glass_hover, p.glass_stroke);

    style.spacing.item_spacing = egui::vec2(space::S8, space::S8);
    style.spacing.button_padding = egui::vec2(space::S16, space::S8);
    style.spacing.interact_size.y = 32.0;
    style.spacing.scroll.bar_width = 8.0;
    style.spacing.scroll.floating = true;
    ctx.set_style_of(which, style);
}

/// Luminancia relativa WCAG y razón de contraste (para las pruebas de SC-008).
#[cfg(test)]
pub fn contrast(a: Color32, b: Color32) -> f32 {
    fn lin(c: u8) -> f32 {
        let c = c as f32 / 255.0;
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    }
    fn lum(c: Color32) -> f32 {
        0.2126 * lin(c.r()) + 0.7152 * lin(c.g()) + 0.0722 * lin(c.b())
    }
    let (la, lb) = (lum(a), lum(b));
    let (hi, lo) = if la > lb { (la, lb) } else { (lb, la) };
    (hi + 0.05) / (lo + 0.05)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(p: &Palette, name: &str) {
        for (label, fg, bg, min) in [
            ("texto/base", p.text, p.bg_base, 4.5),
            ("texto/superficie", p.text, p.bg_surface, 4.5),
            ("texto/elevado", p.text, p.bg_raised, 4.5),
            ("texto/overlay", p.text, p.bg_overlay, 4.5),
            ("texto2/base", p.text_2, p.bg_base, 4.5),
            ("texto2/superficie", p.text_2, p.bg_surface, 4.5),
            ("texto2/elevado", p.text_2, p.bg_raised, 4.5),
            ("texto3/superficie", p.text_3, p.bg_surface, 3.0),
            ("texto3/base", p.text_3, p.bg_base, 3.0),
            ("blanco/relleno", p.on_accent, p.accent_fill, 4.5),
            ("blanco/relleno2", p.on_accent, p.accent_fill_2, 4.5),
            ("acento/superficie", p.accent, p.bg_surface, 3.0),
            ("exito/superficie", p.success, p.bg_surface, 3.0),
            ("peligro/superficie", p.danger, p.bg_surface, 3.0),
        ] {
            let ratio = contrast(fg, bg);
            assert!(ratio >= min, "{name} · {label}: {ratio:.2} < {min}");
        }
    }

    #[test]
    fn dark_palette_meets_wcag_aa() {
        check(&Palette::DARK, "oscuro");
    }

    #[test]
    fn light_palette_meets_wcag_aa() {
        check(&Palette::LIGHT, "claro");
    }

    #[test]
    fn contrast_math_is_correct() {
        assert!((contrast(Color32::BLACK, Color32::WHITE) - 21.0).abs() < 0.01);
        assert!((contrast(Color32::WHITE, Color32::WHITE) - 1.0).abs() < 0.01);
    }
}
