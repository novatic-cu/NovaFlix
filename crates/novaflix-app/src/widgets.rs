//! Componentes de «Nova Glass» (design-system §4).

use egui::{
    epaint::RectShape, vec2, Align2, Color32, ColorImage, Context, CornerRadius, Id, Pos2, Rect,
    Response, Sense, Shape, Stroke, StrokeKind, TextureHandle, TextureOptions, Ui, Vec2,
};

use crate::{
    motion::{self, SNAPPY},
    theme::{self, radius, Palette},
};

#[allow(dead_code)]
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Primary,
    Secondary,
    Ghost,
    Danger,
}

/// Textura de degradado horizontal 64×1 para rellenar formas redondeadas con degradado.
pub fn gradient_texture(ctx: &Context, a: Color32, b: Color32) -> TextureHandle {
    let id = Id::new(("gradient", a, b));
    if let Some(tex) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return tex;
    }
    let pixels = (0..64)
        .map(|i| {
            let t = i as f32 / 63.0;
            let mix = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
            Color32::from_rgb(mix(a.r(), b.r()), mix(a.g(), b.g()), mix(a.b(), b.b()))
        })
        .collect();
    let image = ColorImage {
        size: [64, 1],
        pixels,
        source_size: vec2(64.0, 1.0),
    };
    let tex = ctx.load_texture(format!("grad-{a:?}-{b:?}"), image, TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

/// Degradado vertical de transparente a negro (para dar contraste al texto sobre fotos).
pub fn fade_texture(ctx: &Context) -> TextureHandle {
    let id = Id::new("fade-texture");
    if let Some(tex) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return tex;
    }
    let pixels = (0..64)
        .map(|i| {
            let t = i as f32 / 63.0;
            Color32::from_rgba_unmultiplied(4, 6, 14, (t.powf(1.3) * 235.0) as u8)
        })
        .collect();
    let image = ColorImage {
        size: [1, 64],
        pixels,
        source_size: vec2(1.0, 64.0),
    };
    let tex = ctx.load_texture("fade", image, TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

/// Degradado horizontal de oscuro (izquierda) a casi transparente (derecha): da contraste al
/// texto de la portada destacada sin tapar la imagen del lado derecho.
pub fn fade_h_texture(ctx: &Context) -> TextureHandle {
    let id = Id::new("fade-h-texture");
    if let Some(tex) = ctx.data(|d| d.get_temp::<TextureHandle>(id)) {
        return tex;
    }
    let pixels = (0..64)
        .map(|i| {
            let t = i as f32 / 63.0;
            Color32::from_rgba_unmultiplied(
                8,
                6,
                28,
                (236.0 * (1.0 - t).powf(0.9) + 40.0 * t) as u8,
            )
        })
        .collect();
    let image = ColorImage {
        size: [64, 1],
        pixels,
        source_size: vec2(64.0, 1.0),
    };
    let tex = ctx.load_texture("fade-h", image, TextureOptions::LINEAR);
    ctx.data_mut(|d| d.insert_temp(id, tex.clone()));
    tex
}

/// Fila horizontal desplazable con el comportamiento nativo de `egui`: con el cursor encima,
/// la rueda del ratón la mueve de lado (solo si hay hacia dónde ir; si no, la rueda sigue para
/// la página), la barra y el arrastre con el ratón también funcionan.
pub fn hscroll(ui: &mut Ui, id: &str, show_bar: bool, add: impl FnOnce(&mut Ui)) {
    use egui::scroll_area::{ScrollBarVisibility, ScrollSource};
    ui.scope(|ui| {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 8.0;
        scroll.floating = false;
        ui.style_mut().spacing.scroll = scroll;
        // Una fila solo-horizontal usa la rueda vertical, y solo la «gasta» si se mueve.
        ui.style_mut().always_scroll_the_only_direction = true;
        let visibility = if show_bar {
            ScrollBarVisibility::AlwaysVisible
        } else {
            ScrollBarVisibility::AlwaysHidden
        };
        egui::ScrollArea::horizontal()
            .id_salt(id)
            .auto_shrink([false, true])
            .scroll_bar_visibility(visibility)
            .scroll_source(ScrollSource::ALL)
            .show(ui, |ui| add(ui));
    });
}

/// Caja de alto fijo con scroll vertical propio (rueda, barra y arrastre dentro de la caja).
pub fn vscroll_box(ui: &mut Ui, id: &str, height: f32, add: impl FnOnce(&mut Ui)) {
    use egui::scroll_area::ScrollSource;
    ui.scope(|ui| {
        let mut scroll = egui::style::ScrollStyle::solid();
        scroll.bar_width = 8.0;
        scroll.floating = false;
        ui.style_mut().spacing.scroll = scroll;
        egui::ScrollArea::vertical()
            .id_salt(id)
            .max_height(height)
            .auto_shrink([false, false])
            .scroll_source(ScrollSource::ALL)
            .show(ui, |ui| add(ui));
    });
}

pub fn gradient_rect(ui: &Ui, rect: Rect, rounding: u8, a: Color32, b: Color32) {
    let tex = gradient_texture(ui.ctx(), a, b);
    let uv = Rect::from_min_max(Pos2::new(0.0, 0.0), Pos2::new(1.0, 1.0));
    ui.painter().add(
        RectShape::filled(rect, CornerRadius::same(rounding), Color32::WHITE)
            .with_texture(tex.id(), uv),
    );
}

fn focus_ring(ui: &Ui, p: &Palette, rect: Rect, rounding: u8) {
    ui.painter().rect_stroke(
        rect.expand(3.0),
        CornerRadius::same(rounding.saturating_add(3)),
        Stroke::new(2.0, p.accent.gamma_multiply(0.8)),
        StrokeKind::Outside,
    );
}

/// Botón en cápsula (design-system §4.6).
pub fn button(ui: &mut Ui, p: &Palette, icon: Option<&str>, label: &str, kind: Kind) -> Response {
    let text = match icon {
        Some(icon) if label.is_empty() => icon.to_string(),
        Some(icon) => format!("{icon}  {label}"),
        None => label.to_string(),
    };
    let color = match kind {
        Kind::Primary => p.on_accent,
        Kind::Danger => p.danger,
        Kind::Ghost => p.accent,
        Kind::Secondary => p.text,
    };
    let galley = ui.painter().layout_no_wrap(text, theme::callout(), color);
    let height = if kind == Kind::Primary { 40.0 } else { 36.0 };
    let width = galley.size().x + if label.is_empty() { 22.0 } else { 40.0 };
    let (rect, response) = ui.allocate_exact_size(vec2(width.max(height), height), Sense::click());

    let id = response.id;
    let target = if response.is_pointer_button_down_on() {
        0.97
    } else if response.hovered() {
        1.03
    } else {
        1.0
    };
    let scale = motion::animate(ui.ctx(), id, target, SNAPPY);
    let draw = rect.scale_from_center(scale);

    if ui.is_rect_visible(rect) {
        match kind {
            Kind::Primary => {
                if response.hovered() {
                    ui.painter().rect_stroke(
                        draw.expand(2.0),
                        CornerRadius::same(radius::PILL),
                        Stroke::new(2.0, p.accent.gamma_multiply(0.55)),
                        StrokeKind::Outside,
                    );
                }
                gradient_rect(ui, draw, radius::PILL, p.accent_fill, p.accent_fill_2);
            }
            Kind::Secondary | Kind::Danger => {
                let fill = if response.hovered() {
                    p.glass_hover
                } else {
                    p.glass_fill
                };
                ui.painter().rect(
                    draw,
                    CornerRadius::same(radius::PILL),
                    fill,
                    Stroke::new(1.0, p.glass_stroke),
                    StrokeKind::Inside,
                );
            }
            Kind::Ghost => {
                if response.hovered() {
                    ui.painter()
                        .rect_filled(draw, CornerRadius::same(radius::PILL), p.glass_fill);
                }
            }
        }
        let pos = draw.center() - galley.size() / 2.0;
        ui.painter().galley(pos, galley, color);
        if response.has_focus() {
            focus_ring(ui, p, draw, radius::PILL);
        }
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Chip de filtro (design-system §4.8).
pub fn chip(ui: &mut Ui, p: &Palette, label: &str, active: bool) -> Response {
    let color = if active { p.accent } else { p.text_2 };
    let galley = ui
        .painter()
        .layout_no_wrap(label.to_string(), theme::callout(), color);
    let (rect, response) =
        ui.allocate_exact_size(vec2(galley.size().x + 24.0, 30.0), Sense::click());
    let fill = if active {
        p.accent.gamma_multiply(0.20)
    } else if response.hovered() {
        p.glass_hover
    } else {
        p.glass_fill
    };
    ui.painter().rect(
        rect,
        CornerRadius::same(radius::PILL),
        fill,
        Stroke::new(
            1.0,
            if active {
                p.accent.gamma_multiply(0.5)
            } else {
                p.glass_stroke
            },
        ),
        StrokeKind::Inside,
    );
    ui.painter()
        .galley(rect.center() - galley.size() / 2.0, galley, color);
    if response.has_focus() {
        focus_ring(ui, p, rect, radius::PILL);
    }
    response.on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Control segmentado (Automático · HTTP · FTP…). Devuelve `true` si cambió la selección.
pub fn segmented<T: PartialEq + Copy>(
    ui: &mut Ui,
    p: &Palette,
    current: &mut T,
    options: &[(T, &str)],
) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 6.0;
        for (value, label) in options {
            if chip(ui, p, label, *current == *value).clicked() && *current != *value {
                *current = *value;
                changed = true;
            }
        }
    });
    changed
}

/// Botón circular de icono.
pub fn icon_button(ui: &mut Ui, p: &Palette, icon: &str, tip: &str) -> Response {
    let (rect, response) = ui.allocate_exact_size(vec2(36.0, 36.0), Sense::click());
    if response.hovered() {
        ui.painter()
            .circle_filled(rect.center(), 18.0, p.glass_hover);
    }
    let color = if response.hovered() { p.text } else { p.text_2 };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(18.0),
        color,
    );
    if response.has_focus() {
        focus_ring(ui, p, rect, radius::PILL);
    }
    response
        .on_hover_text(tip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Cabecera de estantería: título a la izquierda y «Ver todo» a la derecha.
pub fn section_header(ui: &mut Ui, p: &Palette, title: &str, action: Option<&str>) -> bool {
    let mut clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(title)
                .font(theme::title2())
                .color(p.text),
        );
        if let Some(action) = action {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                clicked = button(ui, p, None, action, Kind::Ghost).clicked();
            });
        }
    });
    clicked
}

/// Anillo de progreso (design-system §4.9). `frac` en 0..=1; `None` = indeterminado.
pub fn progress_ring(
    ui: &mut Ui,
    p: &Palette,
    frac: Option<f32>,
    size: f32,
    color: Color32,
    center: &str,
) {
    let (rect, _) = ui.allocate_exact_size(Vec2::splat(size), Sense::hover());
    let c = rect.center();
    let r = size / 2.0 - 3.0;
    ui.painter()
        .circle_stroke(c, r, Stroke::new(5.0, p.glass_stroke));
    let sweep = match frac {
        Some(f) => f.clamp(0.0, 1.0) * std::f32::consts::TAU,
        None => {
            ui.ctx().request_repaint();
            std::f32::consts::TAU * 0.28
        }
    };
    if sweep > 0.01 {
        let start = match frac {
            Some(_) => -std::f32::consts::FRAC_PI_2,
            None => (ui.input(|i| i.time) as f32 * 4.0) % std::f32::consts::TAU,
        };
        let steps = ((sweep / std::f32::consts::TAU) * 64.0).ceil().max(2.0) as usize;
        let points: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = start + sweep * (i as f32 / steps as f32);
                c + vec2(a.cos(), a.sin()) * r
            })
            .collect();
        ui.painter()
            .add(Shape::line(points, Stroke::new(5.0, color)));
    }
    if !center.is_empty() {
        let font = if center.chars().count() <= 2 && !center.is_ascii() {
            egui::FontId::proportional(size * 0.42)
        } else {
            egui::FontId::new(size * 0.24, theme::semibold())
        };
        ui.painter()
            .text(c, Align2::CENTER_CENTER, center, font, p.text);
    }
}

/// Esqueleto de carga. Con «reducir movimiento» no pulsa.
pub fn skeleton(ui: &mut Ui, p: &Palette, size: Vec2, rounding: u8) {
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let pulse = if motion::reduce_motion(ui.ctx()) {
        0.5
    } else {
        ui.ctx().request_repaint();
        0.5 + 0.5 * (ui.input(|i| i.time) as f32 * 2.2).sin()
    };
    let color = p.glass_fill.gamma_multiply(0.6 + 0.8 * pulse);
    ui.painter()
        .rect_filled(rect, CornerRadius::same(rounding), color);
}

/// Esqueleto de una tarjeta de título: bloque que pulsa y dos barras de texto abajo.
pub fn skeleton_card(ui: &Ui, p: &Palette, rect: Rect, rounding: CornerRadius) {
    let pulse = if motion::reduce_motion(ui.ctx()) {
        0.5
    } else {
        ui.ctx().request_repaint();
        0.5 + 0.5 * (ui.input(|i| i.time) as f32 * 2.2).sin()
    };
    let base = p.bg_raised.lerp_to_gamma(p.bg_overlay, pulse);
    let bar = p.glass_hover.gamma_multiply(0.55 + 0.45 * pulse);
    ui.painter().rect_filled(rect, rounding, base);
    let pad = 12.0;
    let title = Rect::from_min_size(
        Pos2::new(rect.left() + pad, rect.bottom() - 46.0),
        vec2((rect.width() - 2.0 * pad) * 0.75, 10.0),
    );
    let sub = Rect::from_min_size(
        Pos2::new(rect.left() + pad, rect.bottom() - 26.0),
        vec2((rect.width() - 2.0 * pad) * 0.42, 8.0),
    );
    ui.painter().rect_filled(title, CornerRadius::same(5), bar);
    ui.painter().rect_filled(sub, CornerRadius::same(4), bar);
}

/// Panel de cristal con borde y degradado de reflejo.
pub fn glass_frame(p: &Palette, rounding: u8) -> egui::Frame {
    egui::Frame::new()
        .fill(p.bg_surface.gamma_multiply(0.92))
        .stroke(Stroke::new(1.0, p.glass_stroke))
        .corner_radius(CornerRadius::same(rounding))
        .inner_margin(egui::Margin::same(12))
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: Color32::from_black_alpha(if p.dark { 90 } else { 30 }),
        })
}

/// Punto de estado (verde/ámbar/rojo) con texto de ayuda.
pub fn status_dot(ui: &mut Ui, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(vec2(10.0, 10.0), Sense::hover());
    ui.painter().circle_filled(rect.center(), 4.0, color);
}

#[cfg(test)]
mod tests {
    use super::*;
    use egui::{Event, Modifiers, MouseWheelUnit, RawInput};
    use std::cell::Cell;

    /// Ejecuta `frames` fotogramas con el puntero fijo y, opcionalmente, una rueda en el primero.
    fn run(
        ctx: &Context,
        pointer: Pos2,
        wheel: Option<Vec2>,
        frames: usize,
        mut ui_fn: impl FnMut(&mut Ui),
    ) {
        for n in 0..frames {
            let mut input = RawInput {
                screen_rect: Some(Rect::from_min_size(Pos2::ZERO, vec2(800.0, 700.0))),
                ..Default::default()
            };
            let clock = Id::new("test-clock");
            let t = ctx.data(|d| d.get_temp::<f64>(clock)).unwrap_or(0.0) + 0.016;
            ctx.data_mut(|d| d.insert_temp(clock, t));
            input.time = Some(t);
            input.events.push(Event::PointerMoved(pointer));
            if let (0, Some(delta)) = (n, wheel) {
                input.events.push(Event::MouseWheel {
                    unit: MouseWheelUnit::Line,
                    delta,
                    phase: egui::TouchPhase::Move,
                    modifiers: Modifiers::NONE,
                });
            }
            let mut out = ctx.run_ui(input, |ui| ui_fn(ui));
            out.textures_delta.clear();
        }
    }

    /// Posiciones observadas de la primera tarjeta de cada zona y su rectángulo.
    struct Probe {
        cats_x: Cell<f32>,
        news_x: Cell<f32>,
        box_y: Cell<f32>,
        cats: Cell<Rect>,
        news: Cell<Rect>,
        vbox: Cell<Rect>,
    }

    impl Default for Probe {
        fn default() -> Self {
            Probe {
                cats_x: Cell::new(0.0),
                news_x: Cell::new(0.0),
                box_y: Cell::new(0.0),
                cats: Cell::new(Rect::NOTHING),
                news: Cell::new(Rect::NOTHING),
                vbox: Cell::new(Rect::NOTHING),
            }
        }
    }

    /// Réplica de la estructura de Inicio: categorías (fila), novedades (fila) y una caja vertical.
    fn layout(ui: &mut Ui, probe: &Probe) {
        let top = ui.cursor().top();
        hscroll(ui, "cats", false, |ui| {
            ui.horizontal(|ui| {
                for i in 0..30 {
                    let (r, _) = ui.allocate_exact_size(vec2(120.0, 40.0), Sense::click());
                    if i == 0 {
                        probe.cats_x.set(r.min.x);
                    }
                }
            });
        });
        probe.cats.set(Rect::from_min_max(
            Pos2::new(0.0, top),
            Pos2::new(800.0, ui.cursor().top() - 8.0),
        ));
        ui.add_space(30.0);
        let top = ui.cursor().top();
        hscroll(ui, "news", true, |ui| {
            ui.horizontal(|ui| {
                for i in 0..30 {
                    let (r, _) = ui.allocate_exact_size(vec2(160.0, 100.0), Sense::click());
                    if i == 0 {
                        probe.news_x.set(r.min.x);
                    }
                }
            });
        });
        probe.news.set(Rect::from_min_max(
            Pos2::new(0.0, top),
            Pos2::new(800.0, ui.cursor().top() - 8.0),
        ));
        ui.add_space(30.0);
        let top = ui.cursor().top();
        vscroll_box(ui, "box", 200.0, |ui| {
            for i in 0..40 {
                let (r, _) = ui.allocate_exact_size(vec2(200.0, 50.0), Sense::click());
                if i == 0 {
                    probe.box_y.set(r.min.y);
                }
            }
        });
        probe.vbox.set(Rect::from_min_max(
            Pos2::new(0.0, top),
            Pos2::new(800.0, top + 200.0),
        ));
    }

    /// Prepara el entorno y devuelve (contexto, sondas) con el diseño ya medido.
    fn setup() -> (Context, Probe) {
        let ctx = Context::default();
        let probe = Probe::default();
        run(&ctx, Pos2::new(5.0, 690.0), None, 3, |ui| {
            layout(ui, &probe)
        });
        (ctx, probe)
    }

    fn scroll_over(ctx: &Context, probe: &Probe, at: Pos2) -> (f32, f32, f32) {
        let (c0, n0, b0) = (probe.cats_x.get(), probe.news_x.get(), probe.box_y.get());
        run(ctx, at, None, 2, |ui| layout(ui, probe));
        run(ctx, at, Some(vec2(0.0, -3.0)), 6, |ui| layout(ui, probe));
        (
            probe.cats_x.get() - c0,
            probe.news_x.get() - n0,
            probe.box_y.get() - b0,
        )
    }

    #[test]
    fn wheel_over_categories_moves_only_categories() {
        let (ctx, probe) = setup();
        let (c, n, b) = scroll_over(&ctx, &probe, probe.cats.get().center());
        assert!(c < -10.0, "las categorías deben moverse: {c}");
        assert_eq!((n, b), (0.0, 0.0), "nada más debe moverse");
    }

    #[test]
    fn wheel_over_news_moves_only_news() {
        let (ctx, probe) = setup();
        let (c, n, b) = scroll_over(&ctx, &probe, probe.news.get().center());
        assert!(n < -10.0, "novedades debe moverse: {n}");
        assert_eq!((c, b), (0.0, 0.0), "nada más debe moverse");
    }

    #[test]
    fn wheel_over_the_vertical_box_scrolls_it_down_only() {
        let (ctx, probe) = setup();
        let (c, n, b) = scroll_over(&ctx, &probe, probe.vbox.get().center());
        assert!(b < -10.0, "la caja vertical debe bajar: {b}");
        assert_eq!((c, n), (0.0, 0.0), "las filas no deben moverse");
    }

    #[test]
    fn wheel_over_dead_space_moves_nothing() {
        let (ctx, probe) = setup();
        // El hueco entre las categorías y las novedades no pertenece a ninguna lista.
        let gap = Pos2::new(300.0, probe.cats.get().bottom() + 14.0);
        assert!(!probe.cats.get().contains(gap) && !probe.news.get().contains(gap));
        assert_eq!(scroll_over(&ctx, &probe, gap), (0.0, 0.0, 0.0));
    }
}
