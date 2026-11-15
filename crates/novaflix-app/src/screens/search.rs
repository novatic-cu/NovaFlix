//! Buscar: campo grande, filtros en chips y resultados (tarjetas para carpetas, filas para archivos).

use egui::{vec2, CornerRadius, Margin, RichText, ScrollArea, Stroke, Ui};
use novaflix_core::{Kind, SearchFilter};

use crate::{
    app::NovaApp,
    card::Card,
    theme::{self, radius},
    widgets::{self, Kind as Btn},
};

impl NovaApp {
    fn run_search(&mut self) {
        let query = self.search.query.trim().to_string();
        let filter = SearchFilter {
            kind: match (self.search.only_dirs, self.search.only_files) {
                (true, false) => Some(Kind::Dir),
                (false, true) => Some(Kind::File),
                _ => None,
            },
            ext: self.search.video_only.then(|| "mp4".to_string()),
            ..SearchFilter::default()
        };
        let key = format!("{query}|{:?}|{:?}", filter.kind, filter.ext);
        if key == self.search.last_run {
            return;
        }
        self.search.last_run = key;
        if query.is_empty() {
            self.search.page = None;
            self.search.error = None;
            return;
        }
        match self.core.search(&query, &filter, 120, 0) {
            Ok(page) => {
                self.search.page = Some(page);
                self.search.error = None;
            }
            Err(err) => self.search.error = Some(err.to_string()),
        }
        self.search.recent = self.core.recent_searches().unwrap_or_default();
    }

    pub(crate) fn screen_search(&mut self, ui: &mut Ui) {
        let p = self.p;
        if self.search.recent.is_empty() {
            self.search.recent = self.core.recent_searches().unwrap_or_default();
        }
        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
        ui.label(RichText::new("Buscar").font(theme::title1()).color(p.text));

        // Campo en cápsula.
        let mut focused = false;
        widgets::glass_frame(&p, radius::PILL)
            .inner_margin(Margin::symmetric(22, 6))
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        RichText::new(egui_phosphor::regular::MAGNIFYING_GLASS)
                            .size(22.0)
                            .color(p.text_2),
                    );
                    let edit = egui::TextEdit::singleline(&mut self.search.query)
                        .hint_text("Películas, series, música…")
                        .font(theme::title2())
                        .frame(egui::Frame::NONE)
                        .desired_width(ui.available_width() - 120.0);
                    let response = ui.add(edit);
                    if self.search.focus {
                        response.request_focus();
                        self.search.focus = false;
                    }
                    focused = response.has_focus();
                    if !self.search.query.is_empty()
                        && widgets::icon_button(ui, &p, egui_phosphor::regular::X_CIRCLE, "Limpiar")
                            .clicked()
                    {
                        self.search.query.clear();
                    }
                    ui.label(
                        RichText::new("Ctrl K")
                            .font(theme::caption())
                            .color(p.text_3),
                    );
                });
            });
        if focused {
            let rect = ui.min_rect();
            ui.painter().rect_stroke(
                egui::Rect::from_min_max(
                    rect.left_top() + vec2(0.0, 46.0),
                    rect.left_top() + vec2(rect.width(), 106.0),
                ),
                CornerRadius::same(radius::PILL),
                Stroke::new(0.0, p.accent),
                egui::StrokeKind::Outside,
            );
        }

        // Filtros.
        ui.horizontal(|ui| {
            let all = !self.search.only_dirs && !self.search.only_files;
            if widgets::chip(ui, &p, "Todo", all).clicked() {
                self.search.only_dirs = false;
                self.search.only_files = false;
            }
            if widgets::chip(ui, &p, "Carpetas", self.search.only_dirs).clicked() {
                self.search.only_dirs = !self.search.only_dirs;
                self.search.only_files = false;
            }
            if widgets::chip(ui, &p, "Archivos", self.search.only_files).clicked() {
                self.search.only_files = !self.search.only_files;
                self.search.only_dirs = false;
            }
            if widgets::chip(ui, &p, "Solo vídeo (.mp4)", self.search.video_only).clicked() {
                self.search.video_only = !self.search.video_only;
            }
        });

        self.run_search();

        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            if let Some(error) = self.search.error.clone() {
                ui.label(RichText::new(error).color(p.danger));
                if self.index.is_none()
                    && widgets::button(ui, &p, None, "Preparar el catálogo", Btn::Primary).clicked()
                {
                    self.core.bootstrap(true);
                }
                return;
            }
            let query = self.search.query.trim().to_string();
            if query.is_empty() {
                self.search_empty(ui);
                return;
            }
            let Some(page) = self.search.page.clone() else {
                return;
            };
            ui.label(
                RichText::new(format!("{} resultados · {} ms", page.total, page.took_ms))
                    .font(theme::callout())
                    .color(p.text_2),
            );
            if page.hits.is_empty() {
                widgets::glass_frame(&p, radius::LG).show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.vertical_centered(|ui| {
                        ui.add_space(16.0);
                        ui.label(
                            RichText::new(egui_phosphor::regular::BINOCULARS)
                                .size(36.0)
                                .color(p.text_3),
                        );
                        ui.label(
                            RichText::new(format!("No encontramos «{query}»"))
                                .font(theme::title2()),
                        );
                        ui.label(
                            RichText::new("Prueba con menos palabras o sin el año.")
                                .color(p.text_2),
                        );
                        ui.add_space(16.0);
                    });
                });
                return;
            }
            let cards: Vec<Card> = page.hits.iter().map(Card::from_hit).collect();
            let (dirs, files): (Vec<_>, Vec<_>) = cards.into_iter().partition(|c| c.is_dir);
            if !dirs.is_empty() {
                widgets::section_header(ui, &p, "Carpetas", None);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
                    for card in dirs.iter().take(40) {
                        if self.draw_card(ui, card, NovaApp::card_size()).clicked() {
                            self.open_sheet(&card.path, &card.name, true, None);
                        }
                    }
                });
            }
            if !files.is_empty() {
                widgets::section_header(ui, &p, "Archivos", None);
                for card in files.iter().take(80) {
                    self.file_row(ui, card);
                }
            }
            ui.add_space(24.0);
        });
    }

    fn search_empty(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.add_space(8.0);
        if !self.search.recent.is_empty() {
            ui.label(
                RichText::new("Búsquedas recientes")
                    .font(theme::headline())
                    .color(p.text),
            );
            ui.horizontal_wrapped(|ui| {
                for query in self.search.recent.clone() {
                    if widgets::chip(ui, &p, &query, false).clicked() {
                        self.search.query = query;
                    }
                }
            });
        }
        ui.add_space(8.0);
        ui.label(
            RichText::new("Consejos")
                .font(theme::headline())
                .color(p.text),
        );
        for tip in [
            "Escribe parte del nombre: «wick» encuentra «Wicked».",
            "Los nombres con números por letras también funcionan: «wick3d».",
            "Pulsa Ctrl K desde cualquier pantalla para volver aquí.",
        ] {
            ui.label(
                RichText::new(format!("•  {tip}"))
                    .font(theme::body())
                    .color(p.text_2),
            );
        }
    }
}
