//! Tarjetas de título y portada destacada (design-system §4.2, §4.4).

use egui::{
    epaint::RectShape, text::LayoutJob, vec2, Align2, Color32, CornerRadius, FontId, Pos2, Rect,
    Response, Sense, Stroke, StrokeKind, TextFormat, Ui, UiBuilder, Vec2,
};
use novaflix_core::{catalog, db, index::Entry, norm, Favorite, Hit};

use crate::{
    app::{NovaApp, PosterView},
    cover::Kind as Content,
    format,
    motion::{self, SNAPPY},
    theme::{self, radius},
    widgets::{self, Kind as Btn},
};

#[derive(Clone, Debug)]
pub struct Card {
    pub name: String,
    pub path: String,
    pub is_dir: bool,
    pub size: Option<i64>,
    pub mtime: Option<String>,
}

impl Card {
    pub fn from_entry(e: &Entry) -> Card {
        Card {
            name: e.name.clone(),
            path: e.path.clone(),
            is_dir: e.kind == novaflix_core::Kind::Dir,
            size: e.size,
            mtime: e.mtime.clone(),
        }
    }

    pub fn from_hit(h: &Hit) -> Card {
        Card {
            name: h.name.clone(),
            path: h.path.clone(),
            is_dir: h.kind == novaflix_core::Kind::Dir,
            size: h.size,
            mtime: h.mtime.clone(),
        }
    }

    pub fn from_favorite(f: &Favorite) -> Card {
        Card {
            name: f.label.clone(),
            path: f.path.clone(),
            is_dir: f.kind == novaflix_core::TargetKind::Folder,
            size: None,
            mtime: None,
        }
    }

    pub fn section(&self) -> String {
        catalog::decode_name(&db::section_of(&self.path))
    }

    pub fn year(&self) -> Option<i64> {
        norm::extract_year(&self.name)
    }

    /// Título sin extensión de archivo ni etiquetas de release.
    pub fn title(&self) -> String {
        let base = if self.is_dir {
            self.name.clone()
        } else {
            match self.name.rsplit_once('.') {
                Some((stem, ext)) if ext.len() <= 4 => stem.to_string(),
                _ => self.name.clone(),
            }
        };
        format::clean_title(&base)
    }

    pub fn content(&self) -> Content {
        Content::of_section(&self.section())
    }

    pub fn subtitle(&self) -> String {
        let mut parts: Vec<String> = Vec::new();
        if self.is_dir {
            parts.push(self.content().label().to_string());
        } else {
            let ext = self
                .name
                .rsplit_once('.')
                .map(|(_, e)| e.to_uppercase())
                .unwrap_or_default();
            parts.push(if ext.is_empty() {
                "Archivo".into()
            } else {
                ext
            });
        }
        if let Some(year) = self.year() {
            parts.push(year.to_string());
        }
        if let Some(size) = self.size {
            parts.push(format::human_bytes(size.max(0) as u64));
        }
        parts.join(" · ")
    }
}

pub enum HeroAction {
    Open,
    Favorite,
}

impl NovaApp {
    pub fn card_size() -> Vec2 {
        vec2(160.0, 240.0)
    }

    /// Tarjeta 2:3 con portada generada, icono de tipo, año y título.
    pub fn draw_card(&mut self, ui: &mut Ui, card: &Card, size: Vec2) -> Response {
        let p = self.p;
        let (rect, response) = ui.allocate_exact_size(size, Sense::click());
        if !ui.is_rect_visible(rect) {
            return response;
        }
        let lifted = response.hovered() || response.has_focus();
        let scale = motion::animate(
            ui.ctx(),
            response.id,
            if lifted { 1.03 } else { 1.0 },
            SNAPPY,
        );
        let r = rect.scale_from_center(scale);
        let rounding = CornerRadius::same(radius::MD);

        if lifted {
            ui.painter().add(
                egui::Shadow {
                    offset: [0, 12],
                    blur: 28,
                    spread: 0,
                    color: Color32::from_black_alpha(if p.dark { 150 } else { 60 }),
                }
                .as_shape(r, rounding),
            );
        }
        let view = self.poster_view(&card.path, card.is_dir);
        // Mientras la foto se está pidiendo se pinta un esqueleto (solo de lo que está a la vista).
        if matches!(view, PosterView::Loading) {
            widgets::skeleton_card(ui, &p, r, rounding);
            return response.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        let poster = match view {
            PosterView::Ready(tex, aspect) => Some((tex, aspect)),
            _ => None,
        };
        match poster {
            Some((tex, aspect)) => {
                // Recorte centrado a 2:3 (los pósters ya lo son; las miniaturas 16:9 se recortan).
                let target = size.x / size.y;
                let uv = if aspect > target {
                    let w = target / aspect;
                    Rect::from_min_max(
                        Pos2::new((1.0 - w) / 2.0, 0.0),
                        Pos2::new((1.0 + w) / 2.0, 1.0),
                    )
                } else {
                    let h = aspect / target;
                    Rect::from_min_max(
                        Pos2::new(0.0, (1.0 - h) / 2.0),
                        Pos2::new(1.0, (1.0 + h) / 2.0),
                    )
                };
                ui.painter()
                    .add(RectShape::filled(r, rounding, Color32::WHITE).with_texture(tex, uv));
                // Degradado inferior: el texto blanco siempre se lee sobre la foto.
                let band = Rect::from_min_max(
                    Pos2::new(r.left(), r.bottom() - r.height() * 0.55),
                    r.right_bottom(),
                );
                let fade = widgets::fade_texture(ui.ctx());
                let corners = CornerRadius {
                    nw: 0,
                    ne: 0,
                    sw: radius::MD,
                    se: radius::MD,
                };
                ui.painter().add(
                    RectShape::filled(band, corners, Color32::WHITE).with_texture(
                        fade.id(),
                        Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
                    ),
                );
            }
            None => {
                let tex = self.covers.texture(ui.ctx(), &card.title()).id();
                let uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
                ui.painter()
                    .add(RectShape::filled(r, rounding, Color32::WHITE).with_texture(tex, uv));
            }
        }
        ui.painter().rect_stroke(
            r,
            rounding,
            Stroke::new(1.0, Color32::from_white_alpha(28)),
            StrokeKind::Inside,
        );

        // Icono de tipo (arriba a la izquierda) y año (arriba a la derecha).
        let icon_center = r.left_top() + vec2(24.0, 24.0);
        ui.painter()
            .circle_filled(icon_center, 15.0, Color32::from_black_alpha(100));
        ui.painter().text(
            icon_center,
            Align2::CENTER_CENTER,
            card.content().icon(),
            FontId::proportional(16.0),
            Color32::WHITE,
        );
        if let Some(year) = card.year() {
            let galley =
                ui.painter()
                    .layout_no_wrap(year.to_string(), theme::caption(), Color32::WHITE);
            let chip = Rect::from_min_size(
                Pos2::new(r.right() - galley.size().x - 24.0, r.top() + 10.0),
                galley.size() + vec2(16.0, 10.0),
            );
            ui.painter().rect_filled(
                chip,
                CornerRadius::same(radius::PILL),
                Color32::from_black_alpha(100),
            );
            ui.painter()
                .galley(chip.center() - galley.size() / 2.0, galley, Color32::WHITE);
        }

        // Título (máx. 2 líneas) y subtítulo abajo.
        let mut job = LayoutJob::default();
        job.wrap.max_width = r.width() - 24.0;
        job.wrap.max_rows = 2;
        job.append(
            &card.title(),
            0.0,
            TextFormat {
                font_id: FontId::new(15.0, theme::semibold()),
                color: Color32::WHITE,
                ..Default::default()
            },
        );
        let title = ui.painter().layout_job(job);
        let sub = ui.painter().layout_no_wrap(
            card.subtitle(),
            theme::caption(),
            Color32::from_white_alpha(215),
        );
        let base = r.bottom() - 12.0;
        ui.painter().galley(
            Pos2::new(r.left() + 12.0, base - sub.size().y),
            sub.clone(),
            Color32::WHITE,
        );
        let title_y = base - sub.size().y - 4.0 - title.size().y;
        ui.painter()
            .galley(Pos2::new(r.left() + 12.0, title_y), title, Color32::WHITE);

        if response.has_focus() {
            ui.painter().rect_stroke(
                r.expand(3.0),
                CornerRadius::same(radius::MD + 3),
                Stroke::new(2.0, p.accent),
                StrokeKind::Outside,
            );
        }
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    /// Portada destacada. Devuelve la acción pulsada, si la hay.
    pub fn draw_hero(&mut self, ui: &mut Ui, card: &Card, height: f32) -> Option<HeroAction> {
        let p = self.p;
        let width = ui.available_width();
        let (rect, response) = ui.allocate_exact_size(vec2(width, height), Sense::click());
        let rounding = CornerRadius::same(radius::LG);
        let view = self.poster_view(&card.path, card.is_dir);
        if matches!(view, PosterView::Loading) {
            widgets::skeleton_card(ui, &p, rect, rounding);
            return None;
        }
        let generated = self.covers.texture(ui.ctx(), &card.title()).id();
        let band = Rect::from_min_max(Pos2::new(0.0, 0.10), Pos2::new(1.0, 0.62));
        ui.painter()
            .add(RectShape::filled(rect, rounding, Color32::WHITE).with_texture(generated, band));

        if let PosterView::Ready(ptex, aspect) = view {
            // Fondo ambiental: la propia imagen, ampliada y oscurecida de izquierda a derecha.
            let target = rect.width() / rect.height();
            let uv = if aspect > target {
                let w = target / aspect;
                Rect::from_min_max(
                    Pos2::new((1.0 - w) / 2.0, 0.0),
                    Pos2::new((1.0 + w) / 2.0, 1.0),
                )
            } else {
                let h = aspect / target;
                Rect::from_min_max(Pos2::new(0.0, 0.18), Pos2::new(1.0, (0.18 + h).min(1.0)))
            };
            ui.painter()
                .add(RectShape::filled(rect, rounding, Color32::WHITE).with_texture(ptex, uv));
            let fade = widgets::fade_h_texture(ui.ctx());
            let full = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
            ui.painter().add(
                RectShape::filled(rect, rounding, Color32::WHITE).with_texture(fade.id(), full),
            );

            // Un póster vertical se muestra además nítido a la derecha; las miniaturas
            // horizontales (capturas de episodios) solo sirven de fondo.
            if aspect < 0.9 {
                let h = rect.height() - 48.0;
                let poster_rect = Rect::from_min_size(
                    Pos2::new(rect.right() - 48.0 - h * aspect, rect.top() + 24.0),
                    vec2(h * aspect, h),
                );
                ui.painter().add(
                    egui::Shadow {
                        offset: [0, 14],
                        blur: 30,
                        spread: 0,
                        color: Color32::from_black_alpha(150),
                    }
                    .as_shape(poster_rect, CornerRadius::same(radius::MD)),
                );
                ui.painter().add(
                    RectShape::filled(poster_rect, CornerRadius::same(radius::MD), Color32::WHITE)
                        .with_texture(ptex, full),
                );
            }
        }
        ui.painter().rect_stroke(
            rect,
            rounding,
            Stroke::new(1.0, Color32::from_white_alpha(30)),
            StrokeKind::Inside,
        );

        let mut action = None;
        let compact = height < 260.0;
        let inner = Rect::from_min_max(
            rect.left_bottom() + vec2(32.0, -(height - 24.0).min(226.0)),
            rect.right_bottom() + vec2(-32.0, -18.0),
        );
        {
            // `new_child` (y no `scope_builder`) para no recolocar el cursor del padre hacia atrás.
            let ui = &mut ui.new_child(UiBuilder::new().max_rect(inner));
            ui.vertical(|ui| {
                if compact {
                    ui.spacing_mut().item_spacing.y = 6.0;
                }
                ui.label(
                    egui::RichText::new(format!(
                        "{}  {}",
                        card.content().icon(),
                        card.content().label().to_uppercase()
                    ))
                    .font(theme::caption())
                    .color(Color32::from_white_alpha(230)),
                );
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(card.title())
                            .font(if compact {
                                egui::FontId::new(32.0, theme::semibold())
                            } else {
                                theme::display()
                            })
                            .color(Color32::WHITE),
                    )
                    .truncate(),
                );
                let meta = {
                    let mut parts = vec![card.section()];
                    if let Some(y) = card.year() {
                        parts.push(y.to_string());
                    }
                    if let Some(m) = &card.mtime {
                        parts.push(format!("añadido {}", m.split(' ').next().unwrap_or(m)));
                    }
                    parts.join(" · ")
                };
                ui.label(
                    egui::RichText::new(meta)
                        .font(theme::callout())
                        .color(Color32::from_white_alpha(220)),
                );
                ui.add_space(if compact { 2.0 } else { 10.0 });
                ui.horizontal(|ui| {
                    let label = if card.is_dir {
                        "Ver archivos"
                    } else {
                        "Ver detalles"
                    };
                    if widgets::button(
                        ui,
                        &p,
                        Some(egui_phosphor::regular::PLAY),
                        label,
                        Btn::Primary,
                    )
                    .clicked()
                    {
                        action = Some(HeroAction::Open);
                    }
                    let faved = self.core.is_favorite(
                        if card.is_dir {
                            novaflix_core::TargetKind::Folder
                        } else {
                            novaflix_core::TargetKind::Item
                        },
                        &card.path,
                    );
                    let (icon, text) = if faved {
                        (egui_phosphor::regular::CHECK, "En favoritos")
                    } else {
                        (egui_phosphor::regular::PLUS, "Favoritos")
                    };
                    if widgets::button(ui, &p, Some(icon), text, Btn::Secondary).clicked() {
                        action = Some(HeroAction::Favorite);
                    }
                });
            });
        }
        if action.is_none() && response.clicked() {
            action = Some(HeroAction::Open);
        }
        action
    }
}
