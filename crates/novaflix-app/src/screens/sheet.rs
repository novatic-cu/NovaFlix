//! Ficha de título (panel superpuesto) y diálogo de descarga de carpeta.

use egui::{
    epaint::RectShape, vec2, Align2, Color32, Context, CornerRadius, Id, Order, Pos2, Rect,
    RichText, ScrollArea, Sense, Stroke, StrokeKind, Ui,
};
use novaflix_core::{catalog, db, Level, ListKind, OnExists, TargetKind};

use crate::{
    app::NovaApp,
    card::Card,
    format, motion,
    theme::{self, radius},
    widgets::{self, Kind as Btn},
};

const SHEET_W: f32 = 520.0;

impl NovaApp {
    pub(crate) fn sheet_ui(&mut self, ctx: &Context) {
        let Some(sheet) = self.sheet.as_ref() else {
            return;
        };
        let (path, name, is_dir, size) = (
            sheet.path.clone(),
            sheet.name.clone(),
            sheet.is_dir,
            sheet.size,
        );
        let p = self.p;
        let screen = ctx.content_rect();

        // Velo: un clic fuera cierra la ficha.
        let mut close = false;
        egui::Area::new(Id::new("scrim"))
            .order(Order::Middle)
            .fixed_pos(screen.min)
            .show(ctx, |ui| {
                let response = ui.allocate_rect(screen, Sense::click());
                ui.painter().rect_filled(
                    screen,
                    0.0,
                    Color32::from_black_alpha(if p.dark { 150 } else { 70 }),
                );
                close = response.clicked();
            });

        let card = Card {
            name: name.clone(),
            path: path.clone(),
            is_dir,
            size,
            mtime: None,
        };
        let kind = if is_dir {
            TargetKind::Folder
        } else {
            TargetKind::Item
        };
        let height = (screen.height() - 28.0 - 44.0).max(320.0);
        let slide = motion::animate(ctx, Id::new("sheet-slide"), 1.0, motion::SMOOTH);

        egui::Area::new(Id::new("sheet"))
            .order(Order::Foreground)
            .anchor(Align2::RIGHT_TOP, vec2(-14.0 + (1.0 - slide) * 60.0, 14.0))
            .show(ctx, |ui| {
                let (rect, _) = ui.allocate_exact_size(vec2(SHEET_W, height), Sense::hover());
                let rounding = CornerRadius::same(radius::LG);
                ui.painter().add(
                    egui::Shadow {
                        offset: [0, 24],
                        blur: 64,
                        spread: 0,
                        color: Color32::from_black_alpha(if p.dark { 170 } else { 70 }),
                    }
                    .as_shape(rect, rounding),
                );
                ui.painter().rect(
                    rect,
                    rounding,
                    p.bg_overlay,
                    Stroke::new(1.0, p.glass_stroke),
                    StrokeKind::Inside,
                );
                // Reflejo de luz en el borde superior (elevación 3).
                ui.painter().line_segment(
                    [
                        rect.left_top() + vec2(24.0, 0.5),
                        rect.right_top() + vec2(-24.0, 0.5),
                    ],
                    Stroke::new(1.0, Color32::from_white_alpha(if p.dark { 36 } else { 0 })),
                );

                let mut child = ui.new_child(egui::UiBuilder::new().max_rect(rect.shrink(0.0)));
                let ui = &mut child;
                ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
                    ui.spacing_mut().item_spacing = vec2(12.0, 12.0);

                    // Cabecera con portada ampliada.
                    let header = Rect::from_min_size(ui.cursor().min, vec2(SHEET_W, 200.0));
                    ui.allocate_rect(header, Sense::hover());
                    let tex = self.covers.texture(ui.ctx(), &card.title()).id();
                    let uv = Rect::from_min_max(Pos2::new(0.0, 0.08), Pos2::new(1.0, 0.58));
                    ui.painter().add(
                        RectShape::filled(
                            header,
                            CornerRadius {
                                nw: radius::LG,
                                ne: radius::LG,
                                sw: 0,
                                se: 0,
                            },
                            Color32::WHITE,
                        )
                        .with_texture(tex, uv),
                    );
                    let close_rect = Rect::from_center_size(
                        header.right_top() + vec2(-30.0, 30.0),
                        vec2(40.0, 40.0),
                    );
                    ui.painter().circle_filled(
                        close_rect.center(),
                        18.0,
                        Color32::from_black_alpha(120),
                    );
                    let x = ui.put(close_rect, |ui: &mut Ui| {
                        widgets::icon_button(ui, &p, egui_phosphor::regular::X, "Cerrar (Esc)")
                    });
                    if x.clicked() {
                        close = true;
                    }

                    egui::Frame::new()
                        .inner_margin(egui::Margin::symmetric(24, 8))
                        .show(ui, |ui| {
                            ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                            ui.add(
                                egui::Label::new(
                                    RichText::new(card.title())
                                        .font(theme::title1())
                                        .color(p.text),
                                )
                                .wrap(),
                            );
                            let section = card.section();
                            let mut meta = vec![card.content().label().to_string(), section];
                            if let Some(y) = card.year() {
                                meta.push(y.to_string());
                            }
                            if let Some(s) = size {
                                meta.push(format::human_bytes(s.max(0) as u64));
                            }
                            ui.label(
                                RichText::new(meta.join(" · "))
                                    .font(theme::callout())
                                    .color(p.text_2),
                            );
                            ui.label(
                                RichText::new(catalog::decode_name(path.trim_end_matches('/')))
                                    .font(theme::caption())
                                    .color(p.text_3),
                            );

                            // Acciones.
                            ui.horizontal_wrapped(|ui| {
                                if is_dir {
                                    if widgets::button(
                                        ui,
                                        &p,
                                        Some(egui_phosphor::regular::DOWNLOAD_SIMPLE),
                                        "Descargar carpeta",
                                        Btn::Primary,
                                    )
                                    .clicked()
                                    {
                                        self.core.plan_folder_download(&path);
                                    }
                                } else if widgets::button(
                                    ui,
                                    &p,
                                    Some(egui_phosphor::regular::DOWNLOAD_SIMPLE),
                                    "Descargar",
                                    Btn::Primary,
                                )
                                .clicked()
                                {
                                    self.enqueue(std::slice::from_ref(&path));
                                }
                                if widgets::button(
                                    ui,
                                    &p,
                                    Some(egui_phosphor::regular::PLAY),
                                    "Reproducir",
                                    Btn::Secondary,
                                )
                                .clicked()
                                {
                                    self.play(&path, is_dir);
                                }
                                let faved = self.core.is_favorite(kind, &path);
                                let (icon, label) = if faved {
                                    (egui_phosphor::regular::CHECK, "En favoritos")
                                } else {
                                    (egui_phosphor::regular::STAR, "Favorito")
                                };
                                if widgets::button(ui, &p, Some(icon), label, Btn::Secondary)
                                    .clicked()
                                {
                                    if faved {
                                        let _ = self.core.favorite_remove(kind, &path);
                                    } else {
                                        let _ = self.core.favorite_add(
                                            kind,
                                            &path,
                                            &card.title(),
                                            None,
                                            size,
                                        );
                                    }
                                    self.refresh_library();
                                }
                                self.add_to_list_menu(ui, &path, &card.title());
                                if widgets::button(
                                    ui,
                                    &p,
                                    Some(egui_phosphor::regular::LINK),
                                    "Copiar enlace",
                                    Btn::Ghost,
                                )
                                .clicked()
                                {
                                    ui.ctx().copy_text(self.core.stream_url(&path));
                                    self.toast(Level::Info, "Enlace copiado");
                                }
                            });

                            // Contenido de la carpeta.
                            if is_dir {
                                ui.add_space(6.0);
                                match self.listings.get(&path).cloned() {
                                    None => {
                                        for _ in 0..4 {
                                            widgets::skeleton(
                                                ui,
                                                &p,
                                                vec2(ui.available_width(), 44.0),
                                                radius::SM,
                                            );
                                        }
                                    }
                                    Some(Err(err)) => {
                                        ui.label(RichText::new(err.message).color(p.danger));
                                        if widgets::button(
                                            ui,
                                            &p,
                                            Some(egui_phosphor::regular::ARROW_CLOCKWISE),
                                            "Reintentar",
                                            Btn::Secondary,
                                        )
                                        .clicked()
                                        {
                                            self.request(&path, true);
                                        }
                                    }
                                    Some(Ok(listing)) => {
                                        let cards: Vec<Card> =
                                            listing.entries.iter().map(Card::from_entry).collect();
                                        let (dirs, files): (Vec<_>, Vec<_>) =
                                            cards.into_iter().partition(|c| c.is_dir);
                                        if !dirs.is_empty() {
                                            ui.label(
                                                RichText::new("Subcarpetas")
                                                    .font(theme::headline()),
                                            );
                                            for dir in &dirs {
                                                let row = ui.add(
                                                    egui::Button::new(
                                                        RichText::new(format!(
                                                            "{}  {}",
                                                            egui_phosphor::regular::FOLDER,
                                                            dir.title()
                                                        ))
                                                        .font(theme::body()),
                                                    )
                                                    .frame(false),
                                                );
                                                if row.clicked() {
                                                    self.open_sheet(
                                                        &dir.path, &dir.name, true, None,
                                                    );
                                                }
                                            }
                                        }
                                        if files.is_empty() && dirs.is_empty() {
                                            ui.label(
                                                RichText::new("Esta carpeta está vacía.")
                                                    .color(p.text_2),
                                            );
                                        } else if files.iter().all(|f| !is_video(&f.name))
                                            && !files.is_empty()
                                        {
                                            ui.label(
                                                RichText::new(
                                                    "No hay vídeos aquí; mira las subcarpetas.",
                                                )
                                                .font(theme::callout())
                                                .color(p.text_3),
                                            );
                                        }
                                        if !files.is_empty() {
                                            ui.label(
                                                RichText::new(format!(
                                                    "Archivos ({})",
                                                    files.len()
                                                ))
                                                .font(theme::headline()),
                                            );
                                            for file in &files {
                                                self.file_row(ui, file);
                                            }
                                        }
                                    }
                                }
                            } else {
                                let counter = self.core.counter_for(&path).ok();
                                if let Some(c) = counter.filter(|c| c.attempts > 0) {
                                    ui.label(
                                        RichText::new(format!(
                                            "Descargado {} vez/veces · intentos: {}",
                                            c.completed, c.attempts
                                        ))
                                        .font(theme::caption())
                                        .color(p.text_3),
                                    );
                                }
                            }
                            ui.add_space(24.0);
                        });
                });
            });

        if close {
            self.sheet = None;
        }
    }

    fn add_to_list_menu(&mut self, ui: &mut Ui, path: &str, name: &str) {
        let p = self.p;
        let lists = self.lists.clone();
        let response = widgets::button(
            ui,
            &p,
            Some(egui_phosphor::regular::LIST_PLUS),
            "Lista",
            Btn::Secondary,
        );
        egui::Popup::menu(&response).show(|ui| {
            ui.set_min_width(200.0);
            if ui
                .button(format!("{}  Para ver", egui_phosphor::regular::EYE))
                .clicked()
            {
                let id = self.watchlist_id();
                let _ = self.core.list_add(id, path, Some(name.to_string()));
                self.refresh_library();
                self.toast(Level::Info, "Añadido a «Para ver»");
                ui.close();
            }
            for list in lists.iter().filter(|l| l.kind == ListKind::Playlist) {
                if ui
                    .button(format!("{}  {}", egui_phosphor::regular::LIST, list.name))
                    .clicked()
                {
                    let _ = self.core.list_add(list.id, path, Some(name.to_string()));
                    self.refresh_library();
                    self.toast(Level::Info, format!("Añadido a «{}»", list.name));
                    ui.close();
                }
            }
        });
    }

    /// La lista «Para ver» (se crea la primera vez).
    pub(crate) fn watchlist_id(&mut self) -> i64 {
        if let Some(list) = self.lists.iter().find(|l| l.kind == ListKind::Watchlist) {
            return list.id;
        }
        let id = self
            .core
            .list_create("Para ver", ListKind::Watchlist)
            .unwrap_or(0);
        self.refresh_library();
        id
    }

    pub(crate) fn enqueue(&mut self, paths: &[String]) {
        match self.core.download_enqueue(paths.to_vec(), None) {
            Ok(ids) if !ids.is_empty() => {
                self.refresh_downloads();
                self.toast(Level::Info, "Añadida a la cola de descargas");
            }
            Ok(_) => self.toast(Level::Warn, "No se añadió nada a la cola"),
            Err(err) => self.toast(Level::Error, err.to_string()),
        }
    }

    pub(crate) fn play(&mut self, path: &str, is_dir: bool) {
        if is_dir {
            self.toast(Level::Info, "Elige un archivo de vídeo para reproducirlo");
            return;
        }
        if let Err(err) = self.core.play(path) {
            if err.to_string() == "no-player" {
                self.toast(
                    Level::Warn,
                    "No encontramos VLC ni mpv. Copia el enlace y ábrelo en tu reproductor.",
                );
            } else {
                self.toast(Level::Error, err.to_string());
            }
        }
    }

    /// Diálogo previo a descargar una carpeta completa.
    pub(crate) fn plan_dialog(&mut self, ctx: &Context) {
        let Some((root, result)) = self.plan.clone() else {
            return;
        };
        let p = self.p;
        let mut close = false;
        let mut start = false;
        egui::Window::new("Descargar carpeta")
            .title_bar(false)
            .collapsible(false)
            .resizable(false)
            .anchor(Align2::CENTER_CENTER, vec2(0.0, 0.0))
            .order(Order::Tooltip)
            .frame(
                egui::Frame::new()
                    .fill(p.bg_overlay)
                    .stroke(Stroke::new(1.0, p.glass_stroke))
                    .corner_radius(CornerRadius::same(radius::LG))
                    .inner_margin(egui::Margin::same(24)),
            )
            .show(ctx, |ui| {
                ui.set_width(420.0);
                ui.spacing_mut().item_spacing = vec2(10.0, 10.0);
                ui.label(RichText::new("Descargar carpeta").font(theme::title2()));
                ui.label(
                    RichText::new(catalog::decode_name(&db::name_of(&root)))
                        .font(theme::body())
                        .color(p.text_2),
                );
                match &result {
                    Ok(plan) if plan.files.is_empty() => {
                        ui.label("No hay archivos en esta carpeta.");
                    }
                    Ok(plan) => {
                        ui.label(
                            RichText::new(format!(
                                "{} archivos · {}",
                                plan.files.len(),
                                format::human_bytes(plan.total_bytes)
                            ))
                            .font(theme::headline()),
                        );
                        ui.label(
                            RichText::new(format!(
                                "Destino: {}",
                                self.core.downloads_dir().display()
                            ))
                            .font(theme::caption())
                            .color(p.text_3),
                        );
                    }
                    Err(err) => {
                        ui.label(RichText::new(&err.message).color(p.danger));
                    }
                }
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if let Ok(plan) = &result {
                        if !plan.files.is_empty()
                            && widgets::button(
                                ui,
                                &p,
                                Some(egui_phosphor::regular::DOWNLOAD_SIMPLE),
                                "Empezar",
                                Btn::Primary,
                            )
                            .clicked()
                        {
                            start = true;
                        }
                    }
                    if widgets::button(ui, &p, None, "Cancelar", Btn::Secondary).clicked() {
                        close = true;
                    }
                });
            });
        if start {
            if let Ok(plan) = &result {
                match self.core.download_folder(plan, None, OnExists::Keep) {
                    Ok(ids) => {
                        self.refresh_downloads();
                        self.toast(
                            Level::Info,
                            format!("{} descargas añadidas a la cola", ids.len()),
                        );
                    }
                    Err(err) => self.toast(Level::Error, err.to_string()),
                }
            }
            close = true;
        }
        if close {
            self.plan = None;
        }
    }
}

fn is_video(name: &str) -> bool {
    matches!(
        name.rsplit_once('.')
            .map(|(_, e)| e.to_lowercase())
            .as_deref(),
        Some("mp4" | "mkv" | "avi" | "mov" | "webm" | "m4v")
    )
}
