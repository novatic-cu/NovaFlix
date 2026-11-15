//! Descargas: anillos de progreso, velocidad y acciones.

use egui::{vec2, Align, Layout, RichText, ScrollArea, Sense, Ui};
use novaflix_core::{catalog, db, DownloadRow, DownloadState, Mode};

use crate::{
    app::{NovaApp, Screen},
    format, theme,
    widgets::{self, Kind as Btn},
};

impl NovaApp {
    pub(crate) fn screen_downloads(&mut self, ui: &mut Ui) {
        let p = self.p;
        self.refresh_downloads_if_stale();
        ui.spacing_mut().item_spacing = vec2(12.0, 12.0);

        let rows = self.downloads.clone();
        let active = rows
            .iter()
            .filter(|r| r.state == DownloadState::Active)
            .count();
        let queued = rows
            .iter()
            .filter(|r| r.state == DownloadState::Queued)
            .count();
        let speed: u64 = self.live.values().map(|e| e.speed_bps).sum();

        ui.horizontal(|ui| {
            ui.label(
                RichText::new("Descargas")
                    .font(theme::title1())
                    .color(p.text),
            );
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                if widgets::button(
                    ui,
                    &p,
                    Some(egui_phosphor::regular::BROOM),
                    "Limpiar terminadas",
                    Btn::Secondary,
                )
                .clicked()
                {
                    let _ = self.core.downloads_clear_finished();
                    self.refresh_downloads();
                }
                let summary = if active + queued == 0 {
                    "Sin descargas activas".to_string()
                } else {
                    format!(
                        "{active} activas · {queued} en cola · {}",
                        format::human_speed(speed)
                    )
                };
                ui.label(
                    RichText::new(summary)
                        .font(theme::callout())
                        .color(p.text_2),
                );
            });
        });

        if rows.is_empty() {
            widgets::glass_frame(&p, theme::radius::LG).show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.vertical_centered(|ui| {
                    ui.add_space(24.0);
                    ui.label(
                        RichText::new(egui_phosphor::regular::DOWNLOAD_SIMPLE)
                            .size(40.0)
                            .color(p.text_3),
                    );
                    ui.label(RichText::new("Aún no has descargado nada").font(theme::title2()));
                    ui.label(
                        RichText::new(
                            "Explora el catálogo y pulsa Descargar en cualquier archivo.",
                        )
                        .color(p.text_2),
                    );
                    ui.add_space(8.0);
                    if widgets::button(
                        ui,
                        &p,
                        Some(egui_phosphor::regular::FOLDERS),
                        "Explorar el catálogo",
                        Btn::Primary,
                    )
                    .clicked()
                    {
                        self.go(Screen::Home);
                    }
                    ui.add_space(24.0);
                });
            });
            return;
        }

        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            for row in &rows {
                self.download_row(ui, row);
            }
        });
    }

    fn refresh_downloads_if_stale(&mut self) {
        if self.downloads.is_empty() {
            self.refresh_downloads();
        }
    }

    pub(crate) fn download_row(&mut self, ui: &mut Ui, row: &DownloadRow) {
        let p = self.p;
        let live = self.live.get(&row.id).cloned();
        let (done, total, speed, state) = match &live {
            Some(ev) => (ev.done, ev.total, ev.speed_bps, ev.state),
            None => (
                row.bytes_done.max(0) as u64,
                row.bytes_total.max(0) as u64,
                0,
                row.state,
            ),
        };
        let frac = (total > 0).then(|| (done as f32 / total as f32).clamp(0.0, 1.0));
        let name = catalog::decode_name(&db::name_of(&row.path));
        let proto = match live
            .as_ref()
            .map(|e| e.proto)
            .or_else(|| row.proto.as_deref().map(Mode::parse))
        {
            Some(Mode::Ftp) => "FTP",
            Some(_) => "HTTP",
            None => "",
        };

        widgets::glass_frame(&p, theme::radius::MD).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (color, center) = match state {
                    DownloadState::Done => (p.success, egui_phosphor::regular::CHECK.to_string()),
                    DownloadState::Error => (p.danger, egui_phosphor::regular::WARNING.to_string()),
                    DownloadState::Paused => (p.text_3, egui_phosphor::regular::PAUSE.to_string()),
                    DownloadState::Cancelled => (p.text_3, egui_phosphor::regular::X.to_string()),
                    _ => (
                        p.accent,
                        frac.map(|f| format!("{:.0}%", f * 100.0))
                            .unwrap_or_default(),
                    ),
                };
                let ring_frac = if state == DownloadState::Done {
                    Some(1.0)
                } else {
                    frac
                };
                let indeterminate = matches!(state, DownloadState::Active) && frac.is_none();
                widgets::progress_ring(
                    ui,
                    &p,
                    if indeterminate {
                        None
                    } else {
                        ring_frac.or(Some(0.0))
                    },
                    56.0,
                    color,
                    &center,
                );

                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(crate::format::clean_title(&name))
                                .font(theme::headline())
                                .color(p.text),
                        )
                        .truncate(),
                    );
                    let detail = match state {
                        DownloadState::Done => {
                            format!("Completa · {}", format::human_bytes(total.max(done)))
                        }
                        DownloadState::Error => row.error.clone().unwrap_or_else(|| "Error".into()),
                        DownloadState::Paused => format!(
                            "Pausada · {} de {}",
                            format::human_bytes(done),
                            if total > 0 {
                                format::human_bytes(total)
                            } else {
                                "?".into()
                            }
                        ),
                        DownloadState::Cancelled => "Cancelada".into(),
                        DownloadState::Queued => "En cola".into(),
                        DownloadState::Active => format!(
                            "{} de {} · {} · {}",
                            format::human_bytes(done),
                            if total > 0 {
                                format::human_bytes(total)
                            } else {
                                "?".into()
                            },
                            format::human_speed(speed),
                            format::human_eta(total.saturating_sub(done), speed)
                        ),
                    };
                    ui.label(RichText::new(detail).font(theme::callout()).color(
                        if state == DownloadState::Error {
                            p.danger
                        } else {
                            p.text_2
                        },
                    ));
                    if !proto.is_empty() {
                        ui.label(RichText::new(proto).font(theme::caption()).color(p.text_3));
                    }
                });

                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    use egui_phosphor::regular as i;
                    match state {
                        DownloadState::Active | DownloadState::Queued => {
                            if widgets::icon_button(ui, &p, i::X, "Cancelar").clicked() {
                                self.core.download_cancel(row.id);
                            }
                            if widgets::icon_button(ui, &p, i::PAUSE, "Pausar").clicked() {
                                self.core.download_pause(row.id);
                            }
                        }
                        DownloadState::Paused | DownloadState::Error | DownloadState::Cancelled => {
                            if widgets::icon_button(
                                ui,
                                &p,
                                i::ARROW_CLOCKWISE,
                                "Reanudar / reintentar",
                            )
                            .clicked()
                            {
                                let _ = self.core.download_resume(row.id);
                                self.refresh_downloads();
                            }
                        }
                        DownloadState::Done => {
                            if widgets::icon_button(ui, &p, i::FOLDER_OPEN, "Abrir carpeta")
                                .clicked()
                            {
                                if let Some(dir) = std::path::Path::new(&row.dest).parent() {
                                    let _ = open_folder(dir);
                                }
                            }
                        }
                    }
                });
            });
        });
        let _ = Sense::hover();
    }
}

fn open_folder(dir: &std::path::Path) -> std::io::Result<()> {
    #[cfg(windows)]
    let program = "explorer";
    #[cfg(not(windows))]
    let program = "xdg-open";
    std::process::Command::new(program)
        .arg(dir)
        .spawn()
        .map(|_| ())
}
