//! Inicio: portada destacada, categorías, novedades y estanterías de contenido.

use egui::{vec2, Align, Layout, RichText, Sense, Ui};
use novaflix_core::{DownloadState, TargetKind};

use crate::{
    app::{NovaApp, PosterView, Screen},
    card::{Card, HeroAction},
    cover::Kind as Content,
    format, theme,
    widgets::{self, Kind as Btn},
};

const NEWS: &str = "/Recientes/";

const CHIPS: f32 = 44.0;
/// Cabecera de la estantería + barra de desplazamiento + huecos (sin las tarjetas).
const SHELF_OVERHEAD: f32 = 70.0;
const ROW_GAP: f32 = 12.0;
const BLOCK_GAP: f32 = 16.0;
const HERO_PREFERRED: f32 = 260.0;
const CARD_MIN: f32 = 150.0;
const CARD_MAX: f32 = 230.0;

/// Reparto del alto de la columna principal de Inicio.
#[derive(Debug, PartialEq)]
struct HomeLayout {
    rows: usize,
    card_h: f32,
    hero_h: f32,
}

/// Novedades llena **todo** el alto libre añadiendo filas: se reserva una portada de altura
/// cómoda y con el resto se calculan cuántas filas de tarjetas (150–230 px) caben; la portada
/// se queda con lo que sobre.
fn home_layout(height: f32, wide: bool) -> HomeLayout {
    if !wide {
        return HomeLayout {
            rows: 1,
            card_h: 150.0,
            hero_h: 170.0,
        };
    }
    let room = height - HERO_PREFERRED - CHIPS - SHELF_OVERHEAD - BLOCK_GAP * 3.0;
    let fit = |rows: usize| (room - ROW_GAP * (rows as f32 - 1.0)) / rows as f32;
    let mut rows = ((room + ROW_GAP) / (CARD_MAX + ROW_GAP)).ceil().max(1.0) as usize;
    while rows > 1 && fit(rows) < CARD_MIN {
        rows -= 1;
    }
    let card_h = fit(rows).clamp(CARD_MIN, CARD_MAX);
    let block = SHELF_OVERHEAD + card_h * rows as f32 + ROW_GAP * (rows as f32 - 1.0);
    let hero_h = (height - CHIPS - block - BLOCK_GAP * 3.0).clamp(170.0, 380.0);
    HomeLayout {
        rows,
        card_h,
        hero_h,
    }
}

impl NovaApp {
    /// Entradas de `/Recientes/` ordenadas por fecha, las más nuevas primero.
    fn news(&self) -> Option<Vec<Card>> {
        let Ok(listing) = self.listings.get(NEWS)? else {
            return Some(Vec::new());
        };
        let mut cards: Vec<Card> = listing.entries.iter().map(Card::from_entry).collect();
        cards.sort_by(|a, b| b.mtime.cmp(&a.mtime));
        Some(cards)
    }

    /// Inicio de **altura fija**: no hay scroll de página. Cada zona se mueve sola con la rueda
    /// según dónde esté el cursor: las categorías y Novedades de lado, y «Descubre» hacia abajo
    /// dentro de su propia caja (a la derecha en ventanas anchas, debajo en las estrechas).
    pub(crate) fn screen_home(&mut self, ui: &mut Ui) {
        const GAP: f32 = 16.0;
        const RIGHT_W: f32 = 340.0;
        let p = self.p;
        self.request(NEWS, false);
        ui.spacing_mut().item_spacing = vec2(GAP, GAP);

        // Cabecera: título, descargas en curso y búsqueda.
        ui.horizontal(|ui| {
            ui.label(RichText::new("Inicio").font(theme::title1()).color(p.text));
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(
                    ui,
                    &p,
                    Some(egui_phosphor::regular::MAGNIFYING_GLASS),
                    "Buscar   Ctrl K",
                    Btn::Secondary,
                )
                .clicked()
                {
                    self.go(Screen::Search);
                }
                let active = self
                    .downloads
                    .iter()
                    .filter(|d| matches!(d.state, DownloadState::Active | DownloadState::Queued))
                    .count();
                if active > 0 {
                    let speed: u64 = self.live.values().map(|e| e.speed_bps).sum();
                    let label = format!("{active} descargando · {}", format::human_speed(speed));
                    if widgets::button(
                        ui,
                        &p,
                        Some(egui_phosphor::regular::DOWNLOAD_SIMPLE),
                        &label,
                        Btn::Secondary,
                    )
                    .clicked()
                    {
                        self.go(Screen::Downloads);
                    }
                }
            });
        });

        if let Some(error) = self.index_error.clone() {
            self.home_error(ui, &error);
            return;
        }

        // Dos columnas con rectángulos explícitos (el cursor del padre no se mueve hacia atrás).
        let full = ui.available_rect_before_wrap();
        let wide = full.width() >= 1000.0;
        let left_max_x = if wide {
            full.max.x - RIGHT_W - GAP
        } else {
            full.max.x
        };
        let left = egui::Rect::from_min_max(full.min, egui::pos2(left_max_x, full.max.y));
        let mut left_ui = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(left)
                .layout(Layout::top_down(Align::Min)),
        );
        left_ui.spacing_mut().item_spacing = vec2(GAP, GAP);
        self.home_left(&mut left_ui, left.height(), wide);
        if wide {
            let right =
                egui::Rect::from_min_max(egui::pos2(full.max.x - RIGHT_W, full.min.y), full.max);
            let mut right_ui = ui.new_child(
                egui::UiBuilder::new()
                    .max_rect(right)
                    .layout(Layout::top_down(Align::Min)),
            );
            right_ui.spacing_mut().item_spacing = vec2(GAP, GAP);
            self.home_discover(&mut right_ui, right.height());
        }
        ui.advance_cursor_after_rect(full);
    }

    /// Columna principal: portada, categorías y Novedades (y «Descubre» si la ventana es estrecha).
    fn home_left(&mut self, ui: &mut Ui, height: f32, wide: bool) {
        let p = self.p;
        let news = self.news();

        let HomeLayout {
            rows,
            card_h,
            hero_h,
        } = home_layout(height, wide);

        // Portada destacada (rota entre las 5 novedades más recientes).
        match news.as_ref().filter(|n| !n.is_empty()) {
            Some(cards) => {
                let heroes: Vec<&Card> = cards.iter().filter(|c| c.is_dir).take(5).collect();
                let heroes: Vec<&Card> = if heroes.is_empty() {
                    cards.iter().take(5).collect()
                } else {
                    heroes
                };
                // Se piden a la vez las fotos de las 5 portadas y se rota solo hacia una ya lista:
                // así no aparece un esqueleto cada vez que cambia.
                let ready: Vec<bool> = heroes
                    .iter()
                    .map(|h| !matches!(self.poster_view(&h.path, h.is_dir), PosterView::Loading))
                    .collect();
                if self.hero_since.elapsed().as_secs() >= 8
                    && !crate::motion::reduce_motion(ui.ctx())
                {
                    let len = heroes.len().max(1);
                    if let Some(next) = (1..=len)
                        .map(|k| (self.hero_index + k) % len)
                        .find(|i| ready.get(*i).copied().unwrap_or(false))
                    {
                        self.hero_index = next;
                    }
                    self.hero_since = std::time::Instant::now();
                }
                ui.ctx()
                    .request_repaint_after(std::time::Duration::from_secs(1));
                let hero = heroes[self.hero_index % heroes.len()].clone();
                match self.draw_hero(ui, &hero, hero_h) {
                    Some(HeroAction::Open) => {
                        self.open_sheet(&hero.path, &hero.name, hero.is_dir, hero.size)
                    }
                    Some(HeroAction::Favorite) => {
                        let kind = if hero.is_dir {
                            TargetKind::Folder
                        } else {
                            TargetKind::Item
                        };
                        if self.core.is_favorite(kind, &hero.path) {
                            let _ = self.core.favorite_remove(kind, &hero.path);
                        } else {
                            let _ = self.core.favorite_add(
                                kind,
                                &hero.path,
                                &hero.title(),
                                None,
                                hero.size,
                            );
                        }
                        self.refresh_library();
                    }
                    None => {}
                }
            }
            None => {
                let (rect, _) =
                    ui.allocate_exact_size(vec2(ui.available_width(), hero_h), Sense::hover());
                widgets::skeleton(
                    &mut ui.new_child(egui::UiBuilder::new().max_rect(rect)),
                    &p,
                    rect.size(),
                    theme::radius::LG,
                );
            }
        }

        // Zona 1 · categorías: fila compacta (la rueda las mueve con el cursor encima).
        if !self.sections.is_empty() {
            let sections = self.ordered_sections();
            widgets::hscroll(ui, "categorias", false, |ui| {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    for section in &sections {
                        let name = novaflix_core::catalog::decode_name(&section.name);
                        if self.category_chip(ui, &name).clicked() {
                            self.browse_to(&section.path);
                        }
                    }
                });
            });
        }

        // Zona 2 · Novedades.
        self.news_shelf(ui, &news, card_h, rows);

        // En ventanas estrechas, «Descubre» ocupa el resto, debajo.
        if !wide {
            let rest = ui.available_height();
            if rest > 140.0 {
                self.home_discover(ui, rest);
            }
        }
    }

    /// Zona 3 · «Descubre»: contenido al azar en una caja de alto fijo con scroll vertical propio.
    fn home_discover(&mut self, ui: &mut Ui, height: f32) {
        let p = self.p;
        let cards = self.discover_cards();
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Descubre")
                    .font(theme::title2())
                    .color(p.text),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(ui, &p, egui_phosphor::regular::SHUFFLE, "Mezclar")
                    .clicked()
                {
                    self.shuffle_seed = self
                        .shuffle_seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1442695040888963407);
                    self.discover = None;
                }
            });
        });
        let box_h = (height - 52.0).max(120.0);
        let width = ui.available_width();
        let columns = ((width + 12.0) / (130.0 + 12.0)).floor().max(1.0);
        let card_w = ((width - 14.0 - 12.0 * (columns - 1.0)) / columns).floor();
        let size = vec2(card_w, (card_w * 1.5).round());
        match cards {
            None => {
                widgets::vscroll_box(ui, "descubre", box_h, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                        for _ in 0..6 {
                            widgets::skeleton(ui, &p, size, theme::radius::MD);
                        }
                    });
                });
            }
            Some(cards) => {
                widgets::vscroll_box(ui, "descubre", box_h, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);
                        for card in &cards {
                            if self.draw_card(ui, card, size).clicked() {
                                self.open_sheet(&card.path, &card.name, card.is_dir, card.size);
                            }
                        }
                    });
                });
            }
        }
    }

    /// Mezcla al azar (con la semilla de esta sesión) lo que hay en varias zonas del catálogo.
    /// `None` mientras se cargan los listados.
    fn discover_cards(&mut self) -> Option<Vec<Card>> {
        if let Some(cards) = &self.discover {
            return Some(cards.clone());
        }
        let mut sources: Vec<String> = vec![
            "/Series/Ingles/".into(),
            "/Series/Espanol/".into(),
            "/Documentales/".into(),
            NEWS.into(),
        ];
        // Hasta saber cuál es el año más reciente no se puede completar la lista.
        sources.push(self.latest_year_path("/Peliculas/Extranjeras/")?);
        for source in &sources {
            self.request(source, false);
        }
        if sources.iter().any(|s| !self.listings.contains_key(s)) {
            return None;
        }
        let mut cards: Vec<Card> = sources
            .iter()
            .filter_map(|s| self.listings.get(s)?.as_ref().ok())
            .flat_map(|l| l.entries.iter().map(Card::from_entry))
            .filter(|c| c.is_dir)
            .collect();
        // Fisher–Yates con xorshift64*: determinista para una semilla dada.
        let mut state = self.shuffle_seed | 1;
        let mut next = move || {
            state ^= state >> 12;
            state ^= state << 25;
            state ^= state >> 27;
            state.wrapping_mul(0x2545F4914F6CDD1D)
        };
        for i in (1..cards.len()).rev() {
            let j = (next() % (i as u64 + 1)) as usize;
            cards.swap(i, j);
        }
        cards.truncate(60);
        self.discover = Some(cards.clone());
        Some(cards)
    }

    fn news_shelf(&mut self, ui: &mut Ui, news: &Option<Vec<Card>>, card_h: f32, rows: usize) {
        let p = self.p;
        widgets::section_header(ui, &p, "Novedades", None);
        match news {
            Some(cards) if !cards.is_empty() => {
                self.shelf(
                    ui,
                    "novedades",
                    &cards.iter().take(30 * rows).cloned().collect::<Vec<_>>(),
                    card_h,
                    rows,
                );
            }
            Some(_) => {
                ui.label(RichText::new("No se pudieron cargar las novedades.").color(p.text_2));
                if widgets::button(
                    ui,
                    &p,
                    Some(egui_phosphor::regular::ARROW_CLOCKWISE),
                    "Reintentar",
                    Btn::Secondary,
                )
                .clicked()
                {
                    self.request(NEWS, true);
                }
            }
            None => self.shelf_skeleton(ui, card_h, rows),
        }
    }

    fn home_error(&mut self, ui: &mut Ui, error: &str) {
        let p = self.p;
        widgets::glass_frame(&p, theme::radius::LG).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.add_space(24.0);
                ui.label(
                    RichText::new(egui_phosphor::regular::WIFI_SLASH)
                        .size(40.0)
                        .color(p.danger),
                );
                ui.label(RichText::new("No se pudo preparar el catálogo").font(theme::title2()));
                ui.label(RichText::new(error).font(theme::body()).color(p.text_2));
                if let Some(report) = &self.probe {
                    ui.label(
                        RichText::new(&report.summary)
                            .font(theme::callout())
                            .color(p.text_2),
                    );
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if widgets::button(
                        ui,
                        &p,
                        Some(egui_phosphor::regular::ARROW_CLOCKWISE),
                        "Reintentar",
                        Btn::Primary,
                    )
                    .clicked()
                    {
                        self.index_error = None;
                        self.index_busy = true;
                        self.core.bootstrap(true);
                    }
                    if widgets::button(
                        ui,
                        &p,
                        Some(egui_phosphor::regular::GEAR_SIX),
                        "Ajustes de red",
                        Btn::Secondary,
                    )
                    .clicked()
                    {
                        self.go(Screen::Settings);
                    }
                });
                ui.add_space(24.0);
            });
        });
    }

    /// Estantería horizontal de `rows` filas: barra visible, rueda del ratón y arrastre.
    /// Las tarjetas se reparten por columnas (la 1ª arriba, la 2ª debajo…).
    pub(crate) fn shelf(
        &mut self,
        ui: &mut Ui,
        id: &str,
        cards: &[Card],
        card_h: f32,
        rows: usize,
    ) {
        let size = vec2(card_h * 2.0 / 3.0, card_h);
        widgets::hscroll(ui, id, true, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 16.0;
                for column in cards.chunks(rows.max(1)) {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = 12.0;
                        for card in column {
                            if self.draw_card(ui, card, size).clicked() {
                                self.open_sheet(&card.path, &card.name, card.is_dir, card.size);
                            }
                        }
                    });
                }
            });
            ui.add_space(8.0);
        });
    }

    fn shelf_skeleton(&mut self, ui: &mut Ui, card_h: f32, rows: usize) {
        let p = self.p;
        ui.horizontal(|ui| {
            for _ in 0..6 {
                ui.vertical(|ui| {
                    ui.spacing_mut().item_spacing.y = 12.0;
                    for _ in 0..rows.max(1) {
                        widgets::skeleton(
                            ui,
                            &p,
                            vec2(card_h * 2.0 / 3.0, card_h),
                            theme::radius::MD,
                        );
                    }
                });
            }
        });
    }

    /// Carpeta del año más reciente dentro de `base` (p. ej. `/Peliculas/Extranjeras/2026/`).
    fn latest_year_path(&mut self, base: &str) -> Option<String> {
        self.request(base, false);
        let Ok(listing) = self.listings.get(base)? else {
            return None;
        };
        listing
            .entries
            .iter()
            .filter(|e| e.kind == novaflix_core::Kind::Dir)
            .filter_map(|e| {
                e.name
                    .parse::<u32>()
                    .ok()
                    .map(|year| (year, e.path.clone()))
            })
            .max()
            .map(|(_, path)| path)
    }

    /// Secciones raíz ordenadas por importancia para el usuario.
    fn ordered_sections(&self) -> Vec<novaflix_core::SectionInfo> {
        const FIRST: [&str; 6] = [
            "Peliculas",
            "Series",
            "Infantiles",
            "Documentales",
            "Reality Shows",
            "MP3s",
        ];
        let mut sections = self.sections.clone();
        sections.sort_by_key(|s| {
            (
                FIRST
                    .iter()
                    .position(|n| *n == s.name)
                    .unwrap_or(FIRST.len()),
                std::cmp::Reverse(s.dirs),
            )
        });
        sections
    }

    /// Chip de categoría: icono + nombre en una cápsula.
    fn category_chip(&mut self, ui: &mut Ui, name: &str) -> egui::Response {
        let p = self.p;
        let icon = Content::of_section(name).icon();
        let galley =
            ui.painter()
                .layout_no_wrap(format!("{icon}  {name}"), theme::callout(), p.text);
        let (rect, response) =
            ui.allocate_exact_size(vec2(galley.size().x + 32.0, 40.0), Sense::click());
        let hover = response.hovered() || response.has_focus();
        ui.painter().rect(
            rect,
            egui::CornerRadius::same(theme::radius::PILL),
            if hover { p.glass_hover } else { p.glass_fill },
            egui::Stroke::new(
                1.0,
                if hover {
                    p.accent.gamma_multiply(0.6)
                } else {
                    p.glass_stroke
                },
            ),
            egui::StrokeKind::Inside,
        );
        ui.painter()
            .galley(rect.center() - galley.size() / 2.0, galley, p.text);
        response.on_hover_cursor(egui::CursorIcon::PointingHand)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn used(height: f32) -> (HomeLayout, f32) {
        let l = home_layout(height, true);
        let block = SHELF_OVERHEAD + l.card_h * l.rows as f32 + ROW_GAP * (l.rows as f32 - 1.0);
        let total = l.hero_h + CHIPS + block + BLOCK_GAP * 3.0;
        (l, total)
    }

    #[test]
    fn more_height_means_more_rows() {
        let rows = |h| home_layout(h, true).rows;
        assert_eq!(rows(640.0), 1);
        assert!(
            rows(1000.0) >= 3,
            "una ventana alta debe tener ≥ 3 filas: {}",
            rows(1000.0)
        );
        assert!(rows(1400.0) > rows(1000.0));
    }

    #[test]
    fn the_layout_never_overflows_and_fills_the_height() {
        for height in (600..=1500).step_by(25) {
            let (l, total) = used(height as f32);
            assert!(
                (CARD_MIN..=CARD_MAX).contains(&l.card_h),
                "{height}: tarjeta {}",
                l.card_h
            );
            assert!(total <= height as f32 + 0.5, "{height}: se pasa ({total})");
            // El hueco sin usar es menos que lo que ocupa una fila nueva.
            assert!(
                height as f32 - total < CARD_MIN + ROW_GAP + 100.0,
                "{height}: sobra {}",
                height as f32 - total
            );
        }
    }

    #[test]
    fn narrow_windows_keep_a_single_row() {
        assert_eq!(home_layout(900.0, false).rows, 1);
    }
}
