//! Explorar: migas de pan, carpetas en rejilla y archivos en filas.

use egui::{vec2, Align, Layout, RichText, ScrollArea, Sense, Ui};
use novaflix_core::{catalog, db, ErrorKind};

use crate::{
    app::NovaApp,
    card::Card,
    format, theme,
    widgets::{self, Kind as Btn},
};

impl NovaApp {
    pub(crate) fn screen_browse(&mut self, ui: &mut Ui) {
        let p = self.p;
        let path = self.browse_path.clone();
        self.request(&path, false);
        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);

        // Cabecera: título + migas de pan + actualizar.
        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Explorar")
                    .font(theme::title1())
                    .color(p.text),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::icon_button(
                    ui,
                    &p,
                    egui_phosphor::regular::ARROW_CLOCKWISE,
                    "Actualizar (F5)",
                )
                .clicked()
                {
                    self.request(&path, true);
                }
                if path != "/"
                    && widgets::icon_button(
                        ui,
                        &p,
                        egui_phosphor::regular::ARROW_UP,
                        "Subir un nivel",
                    )
                    .clicked()
                {
                    self.browse_to(
                        &db::parent_of(&path)
                            .trim_end_matches('/')
                            .to_string()
                            .add_slash(),
                    );
                }
            });
        });
        self.breadcrumbs(ui, &path);

        let state = self.listings.get(&path).cloned();
        ScrollArea::vertical()
            .auto_shrink(false)
            .show(ui, |ui| match state {
                None => {
                    ui.horizontal_wrapped(|ui| {
                        for _ in 0..8 {
                            widgets::skeleton(ui, &p, NovaApp::card_size(), theme::radius::MD);
                        }
                    });
                }
                Some(Err(err)) => self.error_panel(ui, &path, err.kind, &err.message),
                Some(Ok(listing)) => {
                    if listing.entries.is_empty() {
                        widgets::glass_frame(&p, theme::radius::LG).show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.vertical_centered(|ui| {
                                ui.add_space(16.0);
                                ui.label(
                                    RichText::new(egui_phosphor::regular::FOLDER_DASHED)
                                        .size(36.0)
                                        .color(p.text_3),
                                );
                                ui.label(
                                    RichText::new("Esta carpeta está vacía").font(theme::title2()),
                                );
                                ui.add_space(16.0);
                            });
                        });
                        return;
                    }
                    if listing.cached {
                        let when = listing.fetched_at.split('T').next().unwrap_or("");
                        ui.label(
                            RichText::new(format!("Guardado el {when} · F5 para actualizar"))
                                .font(theme::caption())
                                .color(p.text_3),
                        );
                    }
                    let cards: Vec<Card> = listing.entries.iter().map(Card::from_entry).collect();
                    let (dirs, files): (Vec<_>, Vec<_>) = cards.into_iter().partition(|c| c.is_dir);
                    if !dirs.is_empty() {
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing = vec2(16.0, 16.0);
                            for card in &dirs {
                                if self.draw_card(ui, card, NovaApp::card_size()).clicked() {
                                    self.browse_to(&card.path);
                                }
                            }
                        });
                    }
                    if !files.is_empty() {
                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                RichText::new(format!("{} archivos", files.len()))
                                    .font(theme::title2()),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if widgets::button(
                                    ui,
                                    &p,
                                    Some(egui_phosphor::regular::DOWNLOAD_SIMPLE),
                                    "Descargar carpeta",
                                    Btn::Secondary,
                                )
                                .clicked()
                                {
                                    self.core.plan_folder_download(&path);
                                }
                            });
                        });
                        for card in &files {
                            self.file_row(ui, card);
                        }
                    }
                    ui.add_space(24.0);
                }
            });
    }

    fn breadcrumbs(&mut self, ui: &mut Ui, path: &str) {
        let p = self.p;
        let parts: Vec<&str> = path
            .trim_matches('/')
            .split('/')
            .filter(|s| !s.is_empty())
            .collect();
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 4.0;
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::HOUSE_LINE),
                "Catálogo",
                Btn::Ghost,
            )
            .clicked()
            {
                self.browse_to("/");
            }
            let mut acc = String::from("/");
            for (i, part) in parts.iter().enumerate() {
                acc.push_str(part);
                acc.push('/');
                ui.label(RichText::new(egui_phosphor::regular::CARET_RIGHT).color(p.text_3));
                let label = catalog::decode_name(part);
                let last = i == parts.len() - 1;
                let target = acc.clone();
                if widgets::button(
                    ui,
                    &p,
                    None,
                    &label,
                    if last { Btn::Secondary } else { Btn::Ghost },
                )
                .clicked()
                    && !last
                {
                    self.browse_to(&target);
                }
            }
        });
    }

    /// Fila de archivo: icono, título, metadatos y acciones.
    pub(crate) fn file_row(&mut self, ui: &mut Ui, card: &Card) {
        let p = self.p;
        let (rect, response) =
            ui.allocate_exact_size(vec2(ui.available_width(), 56.0), Sense::click());
        if response.hovered() {
            ui.painter().rect_filled(
                rect,
                egui::CornerRadius::same(theme::radius::SM),
                p.glass_fill,
            );
        }
        let icon = match card
            .name
            .rsplit_once('.')
            .map(|(_, e)| e.to_lowercase())
            .as_deref()
        {
            Some("mp4" | "mkv" | "avi" | "mov" | "webm") => egui_phosphor::regular::FILM_STRIP,
            Some("srt" | "sub" | "ass" | "vtt") => egui_phosphor::regular::SUBTITLES,
            Some("mp3" | "flac" | "m4a" | "ogg") => egui_phosphor::regular::MUSIC_NOTE,
            Some("pdf" | "epub" | "mobi") => egui_phosphor::regular::BOOK,
            _ => egui_phosphor::regular::FILE,
        };
        ui.painter().text(
            rect.left_center() + vec2(26.0, 0.0),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(22.0),
            p.accent,
        );
        // Dos líneas de una sola fila cada una (con elipsis): nunca se solapan, ni en la ficha estrecha.
        let text_w = (rect.width() - 56.0 - 130.0).max(80.0);
        let one_line = |ui: &Ui, text: String, font: egui::FontId, color: egui::Color32| {
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = text_w;
            job.wrap.max_rows = 1;
            job.append(
                &text,
                0.0,
                egui::TextFormat {
                    font_id: font,
                    color,
                    ..Default::default()
                },
            );
            ui.painter().layout_job(job)
        };
        let title = one_line(
            ui,
            catalog::decode_name(&card.name),
            theme::headline(),
            p.text,
        );
        ui.painter()
            .galley(rect.left_top() + vec2(56.0, 9.0), title, p.text);
        let ext = card
            .name
            .rsplit_once('.')
            .map(|(_, e)| e.to_uppercase())
            .unwrap_or_default();
        let meta = match (&card.mtime, ext.is_empty()) {
            (Some(m), false) => format!("{ext}  ·  {m}"),
            (Some(m), true) => m.clone(),
            (None, false) => ext,
            (None, true) => String::new(),
        };
        let sub = one_line(ui, meta, theme::caption(), p.text_3);
        ui.painter().galley(
            rect.left_bottom() + vec2(56.0, -9.0 - sub.size().y),
            sub,
            p.text_3,
        );
        if let Some(size) = card.size {
            ui.painter().text(
                rect.right_center() + vec2(-70.0, 0.0),
                egui::Align2::RIGHT_CENTER,
                format::human_bytes(size.max(0) as u64),
                theme::callout(),
                p.text_2,
            );
        }
        let button =
            egui::Rect::from_center_size(rect.right_center() + vec2(-28.0, 0.0), vec2(36.0, 36.0));
        let dl = ui.put(button, |ui: &mut Ui| {
            widgets::icon_button(ui, &p, egui_phosphor::regular::DOWNLOAD_SIMPLE, "Descargar")
        });
        if dl.clicked() {
            match self.core.download_enqueue(vec![card.path.clone()], None) {
                Ok(_) => {
                    self.refresh_downloads();
                    self.toast(novaflix_core::Level::Info, "Añadida a la cola de descargas");
                }
                Err(err) => self.toast(novaflix_core::Level::Error, err.to_string()),
            }
        } else if response.clicked() {
            self.open_sheet(&card.path, &card.name, false, card.size);
        }
    }

    pub(crate) fn error_panel(&mut self, ui: &mut Ui, path: &str, kind: ErrorKind, message: &str) {
        let p = self.p;
        let (title, hint) = match kind {
            ErrorKind::Offline => (
                "Sin conexión con el servidor",
                "No se pudo llegar a visuales.uclv.cu. Revisa tu red o tu proxy.",
            ),
            ErrorKind::ProxyAuth => (
                "El proxy rechazó las credenciales",
                "Comprueba el usuario y la contraseña del proxy en Ajustes → Red.",
            ),
            ErrorKind::ProxyBlocked => (
                "Tu proxy no deja pasar FTP",
                "NovaFlix puede usar HTTP, que sí funciona.",
            ),
            ErrorKind::FtpLogin => (
                "El servidor FTP pidió credenciales",
                "Introduce usuario y contraseña en Ajustes → Red.",
            ),
            ErrorKind::NotFound => (
                "Ya no existe",
                "Esa carpeta o archivo ya no está en el servidor.",
            ),
            ErrorKind::Disk => (
                "No se pudo escribir en disco",
                "Comprueba el espacio libre y los permisos.",
            ),
            ErrorKind::Other => ("Algo salió mal", ""),
        };
        widgets::glass_frame(&p, theme::radius::LG).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    RichText::new(egui_phosphor::regular::WARNING_CIRCLE)
                        .size(38.0)
                        .color(p.warning),
                );
                ui.label(RichText::new(title).font(theme::title2()));
                if !hint.is_empty() {
                    ui.label(RichText::new(hint).color(p.text_2));
                }
                ui.label(
                    RichText::new(message)
                        .font(theme::caption())
                        .color(p.text_3),
                );
                ui.add_space(10.0);
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
                        self.request(path, true);
                    }
                    if matches!(
                        kind,
                        ErrorKind::Offline
                            | ErrorKind::ProxyAuth
                            | ErrorKind::ProxyBlocked
                            | ErrorKind::FtpLogin
                    ) && widgets::button(
                        ui,
                        &p,
                        Some(egui_phosphor::regular::GEAR_SIX),
                        "Ajustes de red",
                        Btn::Secondary,
                    )
                    .clicked()
                    {
                        self.go(crate::app::Screen::Settings);
                    }
                });
                ui.add_space(20.0);
            });
        });
    }
}

trait AddSlash {
    fn add_slash(self) -> String;
}
impl AddSlash for String {
    fn add_slash(mut self) -> String {
        if !self.ends_with('/') {
            self.push('/');
        }
        self
    }
}
