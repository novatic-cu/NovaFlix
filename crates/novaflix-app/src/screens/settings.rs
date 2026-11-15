//! Ajustes: Red (protocolo, proxy, diagnóstico), Descargas, Apariencia, Reproductor, Caché y Acerca de.

use egui::{vec2, RichText, ScrollArea, Ui};
use novaflix_core::{IndexMode, Level, Mode, ProxyChoice, Theme, Tri};

use crate::{
    app::NovaApp,
    format, theme,
    widgets::{self, Kind as Btn},
};

#[derive(Clone, Copy, PartialEq)]
enum ProxyKind {
    System,
    Direct,
    Custom,
}

impl NovaApp {
    fn section(&mut self, ui: &mut Ui, title: &str, add: impl FnOnce(&mut Self, &mut Ui)) {
        let p = self.p;
        ui.add_space(8.0);
        ui.label(RichText::new(title).font(theme::title2()).color(p.text));
        widgets::glass_frame(&p, theme::radius::LG)
            .inner_margin(egui::Margin::same(20))
            .show(ui, |ui| {
                ui.set_width(ui.available_width());
                ui.spacing_mut().item_spacing = vec2(10.0, 12.0);
                add(self, ui);
            });
    }

    pub(crate) fn screen_settings(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.label(RichText::new("Ajustes").font(theme::title1()).color(p.text));
        ScrollArea::vertical().auto_shrink(false).show(ui, |ui| {
            ui.set_max_width(820.0);
            self.section(ui, "Red", |app, ui| app.settings_network(ui));
            self.section(ui, "Descargas", |app, ui| app.settings_downloads(ui));
            self.section(ui, "Apariencia", |app, ui| app.settings_appearance(ui));
            self.section(ui, "Reproductor", |app, ui| app.settings_player(ui));
            self.section(ui, "Caché e índice", |app, ui| app.settings_cache(ui));
            self.section(ui, "Acerca de", |app, ui| app.settings_about(ui));
            ui.add_space(32.0);
        });
    }

    fn apply_draft(&mut self) {
        match self.core.save_settings(&self.draft) {
            Ok(()) => {
                self.settings = self.draft.clone();
                self.probing = true;
                self.toast(Level::Info, "Ajustes guardados");
            }
            Err(err) => self.toast(Level::Error, err.to_string()),
        }
    }

    fn settings_network(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.label(RichText::new("Protocolo").font(theme::headline()));
        let mut mode = self.draft.net_mode;
        widgets::segmented(
            ui,
            &p,
            &mut mode,
            &[
                (Mode::Auto, "Automático"),
                (Mode::Http, "HTTP"),
                (Mode::Ftp, "FTP"),
            ],
        );
        self.draft.net_mode = mode;
        let help = match mode {
            Mode::Auto => "Usa FTP si el diagnóstico confirma que funciona desde tu red; si no, HTTP. Un fallo puntual cae a HTTP.",
            Mode::Http => "Funciona en cualquier red, incluso detrás de un proxy corporativo. Admite reanudar descargas.",
            Mode::Ftp => "Solo funciona si tu red llega al puerto 21 del servidor (o vía proxy SOCKS5). Los proxys HTTP no pueden tunelarlo.",
        };
        ui.label(RichText::new(help).font(theme::callout()).color(p.text_2));

        // Aviso si FTP no es viable desde aquí.
        if mode == Mode::Ftp {
            if let Some(report) = self.probe.clone().filter(|r| !r.ftp_usable) {
                widgets::glass_frame(&p, theme::radius::MD).show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(egui_phosphor::regular::WARNING)
                                .size(20.0)
                                .color(p.warning),
                        );
                        ui.add(
                            egui::Label::new(RichText::new(&report.summary).color(p.text)).wrap(),
                        );
                    });
                    if widgets::button(
                        ui,
                        &p,
                        Some(egui_phosphor::regular::CHECK),
                        "Usar HTTP",
                        Btn::Primary,
                    )
                    .clicked()
                    {
                        self.draft.net_mode = Mode::Http;
                        self.apply_draft();
                    }
                });
            }
        }

        ui.separator();
        ui.label(RichText::new("Proxy").font(theme::headline()));
        let mut kind = match &self.draft.proxy {
            ProxyChoice::System => ProxyKind::System,
            ProxyChoice::Direct => ProxyKind::Direct,
            ProxyChoice::Custom(_) => ProxyKind::Custom,
        };
        let before = kind;
        widgets::segmented(
            ui,
            &p,
            &mut kind,
            &[
                (ProxyKind::System, "Del sistema"),
                (ProxyKind::Direct, "Sin proxy"),
                (ProxyKind::Custom, "Personalizado"),
            ],
        );
        if kind != before {
            self.draft.proxy = match kind {
                ProxyKind::System => ProxyChoice::System,
                ProxyKind::Direct => ProxyChoice::Direct,
                ProxyKind::Custom => ProxyChoice::Custom(String::new()),
            };
        }
        match &mut self.draft.proxy {
            ProxyChoice::System => {
                let label = self
                    .core
                    .proxy_label()
                    .unwrap_or_else(|| "ninguno detectado".into());
                ui.label(
                    RichText::new(format!("Se usa el proxy del sistema: {label}"))
                        .font(theme::callout())
                        .color(p.text_2),
                );
            }
            ProxyChoice::Direct => {
                ui.label(
                    RichText::new("Conexión directa; se ignora cualquier proxy.")
                        .font(theme::callout())
                        .color(p.text_2),
                );
            }
            ProxyChoice::Custom(url) => {
                ui.label(
                    RichText::new("URL del proxy (http://, https:// o socks5://)")
                        .font(theme::caption())
                        .color(p.text_3),
                );
                ui.add(
                    egui::TextEdit::singleline(url)
                        .hint_text("http://proxy.ejemplo.cu:3128")
                        .desired_width(380.0),
                );
                ui.label(
                    RichText::new("Usuario")
                        .font(theme::caption())
                        .color(p.text_3),
                );
                ui.add(egui::TextEdit::singleline(&mut self.draft.proxy_user).desired_width(240.0));
                ui.label(
                    RichText::new("Contraseña (solo para esta sesión; no se guarda)")
                        .font(theme::caption())
                        .color(p.text_3),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.proxy_pw)
                        .password(true)
                        .desired_width(240.0),
                );
            }
        }

        ui.separator();
        ui.label(RichText::new("Usuario FTP").font(theme::headline()));
        ui.add(egui::TextEdit::singleline(&mut self.draft.ftp_user).desired_width(240.0));
        ui.label(
            RichText::new(
                "Por defecto «anonymous». Si el servidor pide credenciales aparecerá un aviso.",
            )
            .font(theme::caption())
            .color(p.text_3),
        );

        ui.horizontal(|ui| {
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::CHECK),
                "Aplicar",
                Btn::Primary,
            )
            .clicked()
            {
                if matches!(self.draft.proxy, ProxyChoice::Custom(_)) && !self.proxy_pw.is_empty() {
                    let _ = self.core.set_proxy_password(Some(self.proxy_pw.clone()));
                }
                self.apply_draft();
            }
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::HEARTBEAT),
                "Diagnosticar",
                Btn::Secondary,
            )
            .clicked()
            {
                self.probing = true;
                self.core.run_probe();
            }
        });
        self.diagnostic_ui(ui);
    }

    fn diagnostic_ui(&mut self, ui: &mut Ui) {
        let p = self.p;
        if self.probing {
            ui.horizontal(|ui| {
                widgets::progress_ring(ui, &p, None, 24.0, p.accent, "");
                ui.label(RichText::new("Comprobando la red…").color(p.text_2));
            });
            return;
        }
        let Some(report) = self.probe.clone() else {
            return;
        };
        ui.add_space(4.0);
        let step = |ui: &mut Ui, label: &str, ok: bool, ms: u64, detail: &str, skipped: bool| {
            ui.horizontal(|ui| {
                let (icon, color) = if skipped {
                    (egui_phosphor::regular::MINUS_CIRCLE, p.text_3)
                } else if ok {
                    (egui_phosphor::regular::CHECK_CIRCLE, p.success)
                } else {
                    (egui_phosphor::regular::X_CIRCLE, p.danger)
                };
                ui.label(RichText::new(icon).size(18.0).color(color));
                ui.label(RichText::new(label).font(theme::callout()).color(p.text));
                if !skipped {
                    ui.label(
                        RichText::new(format!("{ms} ms"))
                            .font(theme::caption())
                            .color(p.text_3),
                    );
                }
                ui.add(
                    egui::Label::new(RichText::new(detail).font(theme::caption()).color(p.text_3))
                        .truncate(),
                );
            });
        };
        step(
            ui,
            "DNS",
            report.dns.ok,
            report.dns.ms,
            &report.dns.detail,
            false,
        );
        step(
            ui,
            "FTP (puerto 21)",
            report.ftp.ok,
            report.ftp.ms,
            &report.ftp.detail,
            report.ftp.ms == 0 && !report.ftp.ok && report.ftp.detail.starts_with("sin DNS"),
        );
        if let Some(tunnel) = &report.ftp_via_proxy {
            step(
                ui,
                "Túnel :21 por el proxy",
                tunnel.ok,
                tunnel.ms,
                &tunnel.detail,
                false,
            );
        }
        step(
            ui,
            "HTTPS",
            report.https.ok,
            report.https.ms,
            &report.https.detail,
            false,
        );

        let ok = report.http_usable || report.ftp_usable;
        widgets::glass_frame(&p, theme::radius::MD).show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                let (icon, color) = if ok {
                    (egui_phosphor::regular::SEAL_CHECK, p.success)
                } else {
                    (egui_phosphor::regular::WIFI_SLASH, p.danger)
                };
                ui.label(RichText::new(icon).size(28.0).color(color));
                ui.vertical(|ui| {
                    ui.add(
                        egui::Label::new(
                            RichText::new(&report.summary)
                                .font(theme::headline())
                                .color(p.text),
                        )
                        .wrap(),
                    );
                    if let Some(proxy) = &report.proxy {
                        ui.label(
                            RichText::new(format!("Proxy: {proxy}"))
                                .font(theme::caption())
                                .color(p.text_3),
                        );
                    }
                });
            });
        });
        if widgets::button(
            ui,
            &p,
            Some(egui_phosphor::regular::COPY),
            "Copiar diagnóstico",
            Btn::Ghost,
        )
        .clicked()
        {
            let text = serde_json::to_string_pretty(&report).unwrap_or_default();
            ui.ctx().copy_text(text);
            self.toast(Level::Info, "Diagnóstico copiado (sin contraseñas)");
        }
    }

    fn settings_downloads(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.label(
            RichText::new(format!("Carpeta: {}", self.core.downloads_dir().display()))
                .font(theme::callout())
                .color(p.text_2),
        );
        ui.horizontal(|ui| {
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::FOLDER_OPEN),
                "Cambiar carpeta…",
                Btn::Secondary,
            )
            .clicked()
            {
                if let Some(dir) = rfd::FileDialog::new()
                    .set_directory(self.core.downloads_dir())
                    .pick_folder()
                {
                    self.draft.download_dir = Some(dir);
                    self.apply_draft();
                }
            }
        });
        ui.label(
            RichText::new("Descargas simultáneas (se aplica al reiniciar)")
                .font(theme::caption())
                .color(p.text_3),
        );
        let mut n = self.draft.max_active_downloads;
        widgets::segmented(ui, &p, &mut n, &[(1, "1"), (2, "2"), (3, "3")]);
        if n != self.draft.max_active_downloads {
            self.draft.max_active_downloads = n;
            self.apply_draft();
        }
        ui.label(
            RichText::new("Límite de velocidad")
                .font(theme::caption())
                .color(p.text_3),
        );
        let mut limit = self.draft.speed_limit_kbps;
        widgets::segmented(
            ui,
            &p,
            &mut limit,
            &[
                (0, "Sin límite"),
                (1024, "1 MB/s"),
                (4096, "4 MB/s"),
                (10240, "10 MB/s"),
            ],
        );
        if limit != self.draft.speed_limit_kbps {
            self.draft.speed_limit_kbps = limit;
            self.apply_draft();
        }
    }

    fn settings_appearance(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.label(RichText::new("Tema").font(theme::caption()).color(p.text_3));
        let mut t = self.draft.theme;
        widgets::segmented(
            ui,
            &p,
            &mut t,
            &[
                (Theme::System, "Sistema"),
                (Theme::Dark, "Oscuro"),
                (Theme::Light, "Claro"),
            ],
        );
        if t != self.draft.theme {
            self.draft.theme = t;
            self.settings.theme = t;
            self.apply_draft();
        }
        ui.label(
            RichText::new("Escala de la interfaz")
                .font(theme::caption())
                .color(p.text_3),
        );
        let mut scale = self.draft.ui_scale;
        ui.add(
            egui::Slider::new(&mut scale, 0.8..=2.0)
                .step_by(0.05)
                .fixed_decimals(2),
        );
        if (scale - self.draft.ui_scale).abs() > f32::EPSILON && !ui.input(|i| i.pointer.any_down())
        {
            self.draft.ui_scale = scale;
            self.settings.ui_scale = scale;
            self.apply_draft();
        } else if (scale - self.draft.ui_scale).abs() > f32::EPSILON {
            self.draft.ui_scale = scale;
        }
        ui.label(
            RichText::new("Reducir movimiento")
                .font(theme::caption())
                .color(p.text_3),
        );
        let mut m = self.draft.reduce_motion;
        widgets::segmented(
            ui,
            &p,
            &mut m,
            &[(Tri::System, "Sistema"), (Tri::On, "Sí"), (Tri::Off, "No")],
        );
        if m != self.draft.reduce_motion {
            self.draft.reduce_motion = m;
            self.settings.reduce_motion = m;
            self.apply_draft();
        }
    }

    fn settings_player(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.label(
            RichText::new("Reproductor externo («auto» prueba VLC, mpv, celluloid y totem)")
                .font(theme::caption())
                .color(p.text_3),
        );
        ui.add(egui::TextEdit::singleline(&mut self.draft.player).desired_width(280.0));
        if widgets::button(
            ui,
            &p,
            Some(egui_phosphor::regular::CHECK),
            "Guardar",
            Btn::Secondary,
        )
        .clicked()
        {
            self.apply_draft();
        }
    }

    fn settings_cache(&mut self, ui: &mut Ui) {
        let p = self.p;
        if let Some(stats) = &self.index {
            ui.label(
                RichText::new(format!(
                    "{} carpetas y {} archivos indexados · {} listados guardados ({})",
                    format::thousands(stats.dirs),
                    format::thousands(stats.files),
                    format::thousands(stats.cached_dirs),
                    format::human_bytes(stats.cache_bytes.max(0) as u64)
                ))
                .font(theme::callout())
                .color(p.text_2),
            );
        }
        ui.horizontal_wrapped(|ui| {
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::ARROW_CLOCKWISE),
                "Actualizar catálogo",
                Btn::Secondary,
            )
            .clicked()
            {
                self.index_busy = true;
                self.core.bootstrap(true);
            }
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::BROOM),
                "Vaciar caché de listados",
                Btn::Secondary,
            )
            .clicked()
            {
                let _ = self.core.cache_clear(true);
                self.listings.clear();
                self.toast(Level::Info, "Caché vaciada");
            }
        });
        let deep = self.draft.index_mode == IndexMode::Deep;
        ui.label(RichText::new("Rastreo profundo: indexa también los archivos sueltos (tarda y hace muchas peticiones).").font(theme::caption()).color(p.text_3));
        ui.horizontal(|ui| {
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::TREE_STRUCTURE),
                if deep {
                    "Rastrear ahora"
                } else {
                    "Rastrear ahora (profundo)"
                },
                Btn::Secondary,
            )
            .clicked()
            {
                self.core.crawl_start(250_000);
                self.toast(Level::Info, "Rastreo iniciado en segundo plano");
            }
            if widgets::button(
                ui,
                &p,
                Some(egui_phosphor::regular::STOP),
                "Detener",
                Btn::Ghost,
            )
            .clicked()
            {
                self.core.crawl_stop();
            }
        });
    }

    fn settings_about(&mut self, ui: &mut Ui) {
        let p = self.p;
        ui.label(
            RichText::new(format!("NovaFlix {}", novaflix_core::version())).font(theme::headline()),
        );
        ui.label(
            RichText::new(
                "Explorador de catálogos multimedia públicos. Sin cuentas, sin telemetría.",
            )
            .color(p.text_2),
        );
        ui.label(
            RichText::new("Licencia PolyForm Noncommercial 1.0.0 · github.com/novatic-cu/NovaFlix")
                .font(theme::callout())
                .color(p.text_3),
        );
        ui.label(
            RichText::new(format!("Datos: {}", self.core.data_dir().display()))
                .font(theme::caption())
                .color(p.text_3),
        );
    }
}
