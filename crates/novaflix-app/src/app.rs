//! Estado de la aplicación, bucle de eventos y marco de ventana (dock, barra de estado, avisos).

use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
    time::{Duration, Instant},
};

use egui::{
    vec2, Align, Color32, CornerRadius, Layout, Margin, RichText, Sense, Stroke, StrokeKind,
    UiBuilder,
};
use novaflix_core::{
    Core, DownloadEvent, DownloadRow, Event, Favorite, FolderPlan, Hit, IndexStats, Level,
    ListSummary, Listing, Mode, Notice, Report, SearchPage, SectionInfo, Settings, Theme, Tri,
    UiError,
};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::{
    cover::Covers,
    motion::{self, SMOOTH},
    theme::{self, radius, Palette},
    widgets,
};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Screen {
    Home,
    Search,
    Browse,
    Library,
    Downloads,
    Settings,
}

impl Screen {
    pub fn parse(name: &str) -> Option<Screen> {
        Some(match name {
            "home" => Screen::Home,
            "search" => Screen::Search,
            "browse" => Screen::Browse,
            "library" => Screen::Library,
            "downloads" => Screen::Downloads,
            "settings" => Screen::Settings,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LibTab {
    Favorites,
    Lists,
    Watch,
}

/// Estado del póster de una carpeta: pedido, sin imagen en el servidor, o listo para pintar.
pub enum PosterSlot {
    Pending,
    Missing,
    Ready {
        tex: egui::TextureHandle,
        aspect: f32,
    },
}

/// Lo que debe pintarse para un póster: la foto, un esqueleto (aún se está pidiendo) o la
/// portada generada (la carpeta no tiene imagen).
pub enum PosterView {
    Ready(egui::TextureId, f32),
    Loading,
    Missing,
}

#[derive(Default)]
pub struct SearchState {
    pub query: String,
    pub last_run: String,
    pub page: Option<SearchPage>,
    pub error: Option<String>,
    pub focus: bool,
    pub only_dirs: bool,
    pub only_files: bool,
    pub video_only: bool,
    pub recent: Vec<String>,
}

/// Ficha abierta sobre cualquier pantalla.
pub struct Sheet {
    pub path: String,
    pub name: String,
    pub is_dir: bool,
    pub size: Option<i64>,
}

pub struct Toast {
    pub notice: Notice,
    pub born: Instant,
}

/// Opciones de línea de comandos para depuración y capturas reproducibles.
#[derive(Default, Clone)]
pub struct Cli {
    pub screen: Option<Screen>,
    pub open: Option<String>,
    pub query: Option<String>,
    pub shot: Option<PathBuf>,
    pub light: bool,
    pub tab: Option<String>,
}

pub struct NovaApp {
    pub core: Arc<Core>,
    rx: UnboundedReceiver<Event>,
    pub p: Palette,
    pub settings: Settings,
    pub draft: Settings,
    pub proxy_pw: String,
    pub screen: Screen,
    pub covers: Covers,
    pub posters: HashMap<String, PosterSlot>,
    /// Contenido al azar de «Descubre» y la semilla de la mezcla.
    pub discover: Option<Vec<crate::card::Card>>,
    pub shuffle_seed: u64,

    pub index: Option<IndexStats>,
    pub index_busy: bool,
    pub index_error: Option<String>,
    pub probe: Option<Report>,
    pub probing: bool,

    pub listings: HashMap<String, Result<Listing, UiError>>,
    pub pending: HashSet<String>,
    pub sections: Vec<SectionInfo>,
    pub favorites: Vec<Favorite>,
    pub recent: Vec<Hit>,

    pub search: SearchState,
    pub browse_path: String,
    pub sheet: Option<Sheet>,
    pub plan: Option<(String, Result<FolderPlan, UiError>)>,

    pub downloads: Vec<DownloadRow>,
    pub live: HashMap<i64, DownloadEvent>,

    pub lib_tab: LibTab,
    pub lists: Vec<ListSummary>,
    pub new_list_name: String,

    pub toasts: Vec<Toast>,
    pub hero_index: usize,
    pub hero_since: Instant,

    cli: Cli,
    frames: u32,
    quiet_frames: u32,
    shot_requested: bool,
    wake_at: Option<Instant>,
}

impl NovaApp {
    pub fn new(
        cc: &eframe::CreationContext<'_>,
        core: Arc<Core>,
        rx: UnboundedReceiver<Event>,
        cli: Cli,
    ) -> Self {
        theme::install_fonts(&cc.egui_ctx);
        let mut settings = core.settings().unwrap_or_default();
        if cli.light {
            settings.theme = Theme::Light;
        }
        let system_dark = cc
            .egui_ctx
            .system_theme()
            .map(|t| t == egui::Theme::Dark)
            .unwrap_or(true);
        let p = Palette::of(settings.theme, system_dark);
        theme::apply(&cc.egui_ctx, &p, settings.ui_scale);
        motion::set_reduce_motion(&cc.egui_ctx, settings.reduce_motion == Tri::On);

        let mut app = NovaApp {
            core,
            rx,
            p,
            draft: settings.clone(),
            settings,
            proxy_pw: String::new(),
            screen: cli.screen.unwrap_or(Screen::Home),
            covers: Covers::default(),
            posters: HashMap::new(),
            discover: None,
            shuffle_seed: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos() as u64)
                .unwrap_or(0x9E3779B97F4A7C15),
            index: None,
            index_busy: true,
            index_error: None,
            probe: None,
            probing: true,
            listings: HashMap::new(),
            pending: HashSet::new(),
            sections: Vec::new(),
            favorites: Vec::new(),
            recent: Vec::new(),
            search: SearchState {
                query: cli.query.clone().unwrap_or_default(),
                ..SearchState::default()
            },
            browse_path: "/".into(),
            sheet: None,
            plan: None,
            downloads: Vec::new(),
            live: HashMap::new(),
            lib_tab: match cli.tab.as_deref() {
                Some("lists") => LibTab::Lists,
                Some("watch") => LibTab::Watch,
                _ => LibTab::Favorites,
            },
            lists: Vec::new(),
            new_list_name: String::new(),
            toasts: Vec::new(),
            hero_index: 0,
            hero_since: Instant::now(),
            cli,
            frames: 0,
            quiet_frames: 0,
            shot_requested: false,
            wake_at: None,
        };
        if let Some(notice) = app.core.migration_notice() {
            app.toast(Level::Warn, notice);
        }
        app.core.bootstrap(false);
        app.core.run_probe();
        app.refresh_library();
        app.refresh_downloads();
        if let Some(path) = app.cli.open.clone() {
            let name = novaflix_core::catalog::decode_name(
                path.trim_end_matches('/').rsplit('/').next().unwrap_or(""),
            );
            app.open_sheet(&path, &name, path.ends_with('/'), None);
        }
        if app.screen == Screen::Browse {
            if let Some(path) = app.cli.open.clone().filter(|p| p.ends_with('/')) {
                app.browse_path = path;
            }
        }
        app
    }

    /// Póster de una carpeta si ya está disponible; si no lo conocemos, lo pide (una sola vez).
    pub fn poster_view(&mut self, path: &str, is_dir: bool) -> PosterView {
        match self.posters.get(path) {
            Some(PosterSlot::Ready { tex, aspect }) => PosterView::Ready(tex.id(), *aspect),
            Some(PosterSlot::Missing) => PosterView::Missing,
            Some(PosterSlot::Pending) => PosterView::Loading,
            None => {
                self.posters.insert(path.to_string(), PosterSlot::Pending);
                self.core.poster(path, is_dir);
                PosterView::Loading
            }
        }
    }

    pub fn toast(&mut self, level: Level, message: impl Into<String>) {
        self.toasts.push(Toast {
            notice: Notice {
                level,
                message: message.into(),
            },
            born: Instant::now(),
        });
        if self.toasts.len() > 3 {
            self.toasts.remove(0);
        }
    }

    pub fn refresh_library(&mut self) {
        self.favorites = self.core.favorites().unwrap_or_default();
        self.lists = self.core.lists().unwrap_or_default();
        self.sections = self.core.sections().unwrap_or_default();
        self.recent = self.core.recent_items(24).unwrap_or_default();
    }

    pub fn refresh_downloads(&mut self) {
        self.downloads = self.core.downloads().unwrap_or_default();
    }

    /// Pide una carpeta una sola vez mientras esté en curso.
    pub fn request(&mut self, path: &str, force: bool) {
        if self.pending.contains(path) {
            return;
        }
        if force {
            self.listings.remove(path);
        } else if self.listings.contains_key(path) {
            return;
        }
        self.pending.insert(path.to_string());
        self.core.browse(path, force);
    }

    pub fn open_sheet(&mut self, path: &str, name: &str, is_dir: bool, size: Option<i64>) {
        self.sheet = Some(Sheet {
            path: path.to_string(),
            name: name.to_string(),
            is_dir,
            size,
        });
        if is_dir {
            self.request(path, false);
        }
    }

    pub fn go(&mut self, screen: Screen) {
        self.screen = screen;
        if screen == Screen::Search {
            self.search.focus = true;
        }
        if matches!(screen, Screen::Library | Screen::Home) {
            self.refresh_library();
        }
    }

    pub fn browse_to(&mut self, path: &str) {
        self.browse_path = path.to_string();
        self.screen = Screen::Browse;
        self.request(path, false);
    }

    fn handle_events(&mut self, ctx: &egui::Context) {
        while let Ok(event) = self.rx.try_recv() {
            match event {
                Event::IndexProgress { .. } => self.index_busy = true,
                Event::IndexReady(stats) => {
                    self.index = Some(stats);
                    self.index_busy = false;
                    self.index_error = None;
                    self.refresh_library();
                }
                Event::CrawlProgress { .. } => {}
                Event::Listing { path, result } => {
                    self.pending.remove(&path);
                    self.listings.insert(path, result);
                }
                Event::FolderPlan { root, result } => self.plan = Some((root, result)),
                Event::Poster { path, image } => {
                    let slot = match image {
                        Some(img) => {
                            let size = [img.width as usize, img.height as usize];
                            let color = egui::ColorImage::from_rgba_unmultiplied(size, &img.rgba);
                            let tex = ctx.load_texture(
                                format!("poster-{path}"),
                                color,
                                egui::TextureOptions::LINEAR,
                            );
                            PosterSlot::Ready {
                                tex,
                                aspect: img.width as f32 / img.height as f32,
                            }
                        }
                        None => PosterSlot::Missing,
                    };
                    self.posters.insert(path, slot);
                }
                Event::Probe(report) => {
                    self.probing = false;
                    self.probe = Some(report);
                }
                Event::Download(ev) => {
                    let finished = ev.state.is_finished();
                    let id = ev.id;
                    if finished {
                        self.live.remove(&id);
                        self.refresh_downloads();
                        if ev.state == novaflix_core::DownloadState::Done {
                            self.toast(Level::Info, "Descarga completa");
                        } else if let Some(err) = &ev.error {
                            self.toast(Level::Error, err.message.clone());
                        }
                    } else {
                        self.live.insert(id, ev);
                    }
                }
                Event::Notice(notice) => {
                    if notice.level == Level::Error && self.index_busy {
                        self.index_busy = false;
                        self.index_error = Some(notice.message.clone());
                    }
                    self.toast(notice.level, notice.message);
                }
            }
            ctx.request_repaint();
        }
        self.toasts.retain(|t| {
            t.notice.level == Level::Error || t.born.elapsed() < Duration::from_secs(4)
        });
    }

    fn apply_theme(&mut self, ctx: &egui::Context) {
        let system_dark = ctx
            .system_theme()
            .map(|t| t == egui::Theme::Dark)
            .unwrap_or(true);
        let wanted = if self.cli.light {
            Palette::LIGHT
        } else {
            Palette::of(self.settings.theme, system_dark)
        };
        if wanted.dark != self.p.dark {
            self.p = wanted;
        }
        theme::apply(ctx, &self.p, self.settings.ui_scale);
        let reduce = match self.settings.reduce_motion {
            Tri::On => true,
            Tri::Off => false,
            Tri::System => false,
        };
        motion::set_reduce_motion(ctx, reduce);
    }

    /// Mientras la ventana de capturas espera, decide cuándo hacer la captura y salir.
    fn drive_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.cli.shot.clone() else {
            return;
        };
        self.frames += 1;
        let posters_loading = self
            .posters
            .values()
            .any(|p| matches!(p, PosterSlot::Pending));
        let idle = self.pending.is_empty() && !self.index_busy && !self.probing && !posters_loading;
        self.quiet_frames = if idle { self.quiet_frames + 1 } else { 0 };
        // `NOVAFLIX_SHOT_AT=N` fuerza la captura en el fotograma N (depuración de primer pintado).
        let forced = std::env::var("NOVAFLIX_SHOT_AT")
            .ok()
            .and_then(|v| v.parse::<u32>().ok());
        let due = match forced {
            Some(n) => self.frames >= n,
            None => (self.quiet_frames > 40 && self.frames > 60) || self.frames > 2400,
        };
        if !self.shot_requested && due {
            self.shot_requested = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
        }
        let image = ctx.input(|i| {
            i.events.iter().find_map(|e| match e {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        if let Some(image) = image {
            if let Err(err) = crate::shot::save_png(&path, &image) {
                eprintln!("no se pudo guardar la captura: {err}");
            }
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
        ctx.request_repaint_after(Duration::from_millis(50));
    }

    fn global_shortcuts(&mut self, ctx: &egui::Context) {
        use egui::{Key, Modifiers};
        let ctrl = |key| ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, key));
        if ctrl(Key::K) {
            self.go(Screen::Search);
        }
        for (key, screen) in [
            (Key::Num1, Screen::Home),
            (Key::Num2, Screen::Search),
            (Key::Num3, Screen::Browse),
            (Key::Num4, Screen::Library),
            (Key::Num5, Screen::Downloads),
            (Key::Comma, Screen::Settings),
        ] {
            if ctrl(key) {
                self.go(screen);
                if screen == Screen::Browse {
                    let path = self.browse_path.clone();
                    self.request(&path, false);
                }
            }
        }
        if ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape)) {
            if self.plan.is_some() {
                self.plan = None;
            } else if self.sheet.is_some() {
                self.sheet = None;
            }
        }
        let refresh = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F5))
            || ctx.input_mut(|i| i.consume_key(Modifiers::COMMAND, Key::R));
        if refresh && self.screen == Screen::Browse {
            let path = self.browse_path.clone();
            self.request(&path, true);
        }
        for (key, delta) in [(Key::Equals, 0.1_f32), (Key::Minus, -0.1)] {
            if ctrl(key) {
                self.settings.ui_scale = (self.settings.ui_scale + delta).clamp(0.8, 2.0);
                self.draft.ui_scale = self.settings.ui_scale;
                let _ = self.core.save_settings(&self.settings);
            }
        }
        if ctrl(Key::Num0) {
            self.settings.ui_scale = 1.0;
            self.draft.ui_scale = 1.0;
            let _ = self.core.save_settings(&self.settings);
        }
    }

    // ---- Marco de ventana ----------------------------------------------------------------

    fn dock(&mut self, ui: &mut egui::Ui) {
        use egui_phosphor::regular as i;
        let items = [
            (Screen::Home, i::HOUSE, "Inicio"),
            (Screen::Search, i::MAGNIFYING_GLASS, "Buscar"),
            (Screen::Browse, i::FOLDERS, "Explorar"),
            (Screen::Library, i::BOOKMARKS_SIMPLE, "Biblioteca"),
            (Screen::Downloads, i::DOWNLOAD_SIMPLE, "Descargas"),
        ];
        let p = self.p;
        egui::Frame::new()
            .fill(p.bg_surface)
            .stroke(Stroke::new(1.0, p.glass_stroke))
            .corner_radius(CornerRadius::same(radius::XL))
            .inner_margin(Margin::symmetric(10, 14))
            .shadow(egui::Shadow {
                offset: [0, 10],
                blur: 28,
                spread: 0,
                color: Color32::from_black_alpha(if p.dark { 110 } else { 36 }),
            })
            .show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                // Marca
                let (rect, _) = ui.allocate_exact_size(vec2(44.0, 44.0), Sense::hover());
                widgets::gradient_rect(ui, rect, 14, p.accent_fill, p.accent_fill_2);
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "N",
                    egui::FontId::new(24.0, theme::semibold()),
                    p.on_accent,
                );
                ui.add_space(6.0);
                for (screen, icon, tip) in items {
                    if self.dock_button(ui, screen, icon, tip).clicked() {
                        self.go(screen);
                        if screen == Screen::Browse {
                            let path = self.browse_path.clone();
                            self.request(&path, false);
                        }
                    }
                }
                let remaining = (ui.available_height() - 56.0).max(0.0);
                ui.add_space(remaining);
                if self
                    .dock_button(ui, Screen::Settings, i::GEAR_SIX, "Ajustes")
                    .clicked()
                {
                    self.go(Screen::Settings);
                }
            });
    }

    fn dock_button(
        &mut self,
        ui: &mut egui::Ui,
        screen: Screen,
        icon: &str,
        tip: &str,
    ) -> egui::Response {
        let p = self.p;
        let active = self.screen == screen;
        let (rect, response) = ui.allocate_exact_size(vec2(48.0, 48.0), Sense::click());
        let level = motion::animate(
            ui.ctx(),
            response.id.with("active"),
            if active { 1.0 } else { 0.0 },
            SMOOTH,
        );
        let hover = response.hovered();
        let base = if hover {
            p.glass_hover
        } else {
            Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, CornerRadius::same(16), base);
        if level > 0.01 {
            ui.painter().rect_filled(
                rect,
                CornerRadius::same(16),
                p.accent.gamma_multiply(0.22 * level),
            );
        }
        let color = if active {
            p.accent
        } else if hover {
            p.text
        } else {
            p.text_2
        };
        ui.painter().text(
            rect.center(),
            egui::Align2::CENTER_CENTER,
            icon,
            egui::FontId::proportional(22.0),
            color,
        );
        if response.has_focus() {
            ui.painter().rect_stroke(
                rect.expand(2.0),
                CornerRadius::same(18),
                Stroke::new(2.0, p.accent),
                StrokeKind::Outside,
            );
        }
        response
            .on_hover_text(tip)
            .on_hover_cursor(egui::CursorIcon::PointingHand)
    }

    fn status_bar(&mut self, ui: &mut egui::Ui) {
        let p = self.p;
        let mode = self.core.active_mode();
        let proxy = self.core.proxy_label();
        let (color, state) = match (&self.probe, self.probing) {
            (_, true) => (p.warning, "Comprobando la red…"),
            (Some(r), _) if r.http_usable || r.ftp_usable => (p.success, "Conectado"),
            (Some(_), _) => (p.danger, "Sin conexión"),
            (None, _) => (p.warning, "Comprobando la red…"),
        };
        let mut open_settings = false;
        ui.horizontal_centered(|ui| {
            ui.add_space(16.0);
            let response = ui
                .horizontal(|ui| {
                    widgets::status_dot(ui, color);
                    let proto = match mode {
                        Mode::Ftp => "FTP",
                        _ => "HTTP",
                    };
                    ui.label(
                        RichText::new(format!("{state} · {proto}"))
                            .font(theme::caption())
                            .color(p.text_2),
                    );
                    if let Some(proxy) = &proxy {
                        ui.label(
                            RichText::new(format!("proxy {proxy}"))
                                .font(theme::caption())
                                .color(p.text_3),
                        );
                    }
                })
                .response
                .interact(Sense::click())
                .on_hover_text("Abrir ajustes de red")
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            open_settings = response.clicked();
            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                ui.add_space(16.0);
                let text = if self.index_busy {
                    "Preparando el catálogo…".to_string()
                } else if let Some(stats) = &self.index {
                    format!(
                        "{} carpetas · {} archivos indexados",
                        crate::format::thousands(stats.dirs),
                        crate::format::thousands(stats.files)
                    )
                } else {
                    String::new()
                };
                ui.label(RichText::new(text).font(theme::caption()).color(p.text_3));
            });
        });
        if open_settings {
            self.go(Screen::Settings);
        }
    }

    fn toasts_ui(&mut self, ctx: &egui::Context) {
        let p = self.p;
        let mut dismiss: Option<usize> = None;
        egui::Area::new(egui::Id::new("toasts"))
            .order(egui::Order::Tooltip)
            .anchor(egui::Align2::RIGHT_BOTTOM, vec2(-20.0, -44.0))
            .show(ctx, |ui| {
                ui.spacing_mut().item_spacing.y = 8.0;
                for (index, toast) in self.toasts.iter().enumerate() {
                    use egui_phosphor::regular as i;
                    let (icon, color) = match toast.notice.level {
                        Level::Info => (i::CHECK_CIRCLE, p.success),
                        Level::Warn => (i::WARNING, p.warning),
                        Level::Error => (i::X_CIRCLE, p.danger),
                    };
                    widgets::glass_frame(&p, radius::MD).show(ui, |ui| {
                        ui.set_max_width(360.0);
                        ui.horizontal(|ui| {
                            ui.label(RichText::new(icon).size(18.0).color(color));
                            ui.add(
                                egui::Label::new(
                                    RichText::new(&toast.notice.message)
                                        .font(theme::callout())
                                        .color(p.text),
                                )
                                .wrap(),
                            );
                            if toast.notice.level == Level::Error
                                && widgets::icon_button(ui, &p, i::X, "Cerrar").clicked()
                            {
                                dismiss = Some(index);
                            }
                        });
                    });
                }
            });
        if let Some(index) = dismiss {
            self.toasts.remove(index);
        }
        if !self.toasts.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(250));
        }
    }
}

impl eframe::App for NovaApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.handle_events(ctx);
        self.apply_theme(ctx);
        self.global_shortcuts(ctx);
        self.drive_screenshot(ctx);
        if let Some(at) = self.wake_at {
            if Instant::now() >= at {
                self.wake_at = None;
            }
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let p = self.p;
        let ctx = ui.ctx().clone();

        egui::Panel::bottom("status")
            .exact_size(28.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(p.bg_base))
            .show(ui, |ui| self.status_bar(ui));

        egui::Panel::left("dock")
            .exact_size(92.0)
            .resizable(false)
            .show_separator_line(false)
            .frame(egui::Frame::new().fill(p.bg_base).inner_margin(Margin {
                left: 16,
                right: 0,
                top: 16,
                bottom: 8,
            }))
            .show(ui, |ui| self.dock(ui));

        egui::CentralPanel::no_frame().show(ui, |ui| {
            // Fondo plano con un resplandor tenue de acento en la esquina (elevación 0).
            let rect = ui.max_rect();
            ui.painter().rect_filled(rect, 0.0, p.bg_base);
            ui.painter().circle_filled(
                rect.right_top() + vec2(-80.0, 40.0),
                360.0,
                p.accent.gamma_multiply(if p.dark { 0.035 } else { 0.05 }),
            );

            let content = rect.shrink2(vec2(48.0, 16.0));
            ui.scope_builder(UiBuilder::new().max_rect(content), |ui| match self.screen {
                Screen::Home => self.screen_home(ui),
                Screen::Search => self.screen_search(ui),
                Screen::Browse => self.screen_browse(ui),
                Screen::Library => self.screen_library(ui),
                Screen::Downloads => self.screen_downloads(ui),
                Screen::Settings => self.screen_settings(ui),
            });
        });

        self.sheet_ui(&ctx);
        self.plan_dialog(&ctx);
        self.toasts_ui(&ctx);
    }
}
