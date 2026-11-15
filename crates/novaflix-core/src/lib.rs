//! Núcleo de NovaFlix: catálogo, caché, búsqueda, transporte HTTP/FTP, descargas y biblioteca.
//! No depende de ningún toolkit gráfico (Constitución II). Contratos en `specs/001-novaflix-v2/contracts/`.

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc, Mutex, RwLock},
};

use rusqlite::Connection;
use tokio::sync::{mpsc::UnboundedSender, Semaphore};

pub mod catalog;
pub mod db;
pub mod downloads;
pub mod error;
pub mod index;
pub mod library;
pub mod migrate;
pub mod norm;
pub mod posters;
pub mod settings;
pub mod transport;

pub use downloads::{DownloadEvent, DownloadRow, DownloadState, FolderPlan, OnExists, PlannedFile};
pub use error::{Error, ErrorKind, Result, UiError};
pub use index::{Hit, IndexStats, Kind, Listing, SearchFilter, SearchPage, SectionInfo};
pub use library::{Counter, Favorite, ListItem, ListKind, ListSummary, TargetKind};
pub use posters::PosterImage;
pub use settings::{IndexMode, Settings, Theme, Tri};
pub use transport::{probe::Report, Mode, Net, NetConfig, ProxyChoice};

use error::lock;

pub fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Warn,
    Error,
}

#[derive(Debug, Clone)]
pub struct Notice {
    pub level: Level,
    pub message: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Progress {
    Start,
    Done,
}

/// Eventos núcleo → interfaz (contracts/events.md).
#[derive(Debug, Clone)]
pub enum Event {
    IndexProgress {
        state: Progress,
        dirs: Option<usize>,
    },
    IndexReady(IndexStats),
    CrawlProgress {
        dirs: usize,
        queued: usize,
        files: usize,
        errors: usize,
        max: usize,
    },
    Listing {
        path: String,
        result: std::result::Result<Listing, UiError>,
    },
    FolderPlan {
        root: String,
        result: std::result::Result<FolderPlan, UiError>,
    },
    Probe(Report),
    /// Póster de una carpeta (`None` = no tiene imagen o no se pudo obtener).
    Poster {
        path: String,
        image: Option<PosterImage>,
    },
    Download(DownloadEvent),
    Notice(Notice),
}

pub struct CoreOptions {
    pub data_dir: PathBuf,
    pub events: UnboundedSender<Event>,
    pub wake: Arc<dyn Fn() + Send + Sync>,
    pub runtime: tokio::runtime::Handle,
    /// Raíz del catálogo. `None` = el sitio oficial. Solo para pruebas.
    pub base_url: Option<String>,
}

pub struct Core {
    pub(crate) cache: Mutex<Connection>,
    pub(crate) app: Mutex<Connection>,
    pub(crate) net: RwLock<Arc<Net>>,
    pub(crate) events: UnboundedSender<Event>,
    pub(crate) wake: Arc<dyn Fn() + Send + Sync>,
    pub(crate) rt: tokio::runtime::Handle,
    pub(crate) data_dir: PathBuf,
    pub(crate) base_url: String,
    pub(crate) cancels: Mutex<HashMap<i64, Arc<downloads::Control>>>,
    pub(crate) dl_sem: Arc<Semaphore>,
    pub(crate) crawl_stop: Arc<AtomicBool>,
    pub(crate) poster_sem: Arc<Semaphore>,
    /// Contraseña del proxy: solo en memoria hasta que exista integración con el llavero (T-044).
    pub(crate) proxy_password: Mutex<Option<String>>,
    pub(crate) migration_notice: Option<String>,
}

impl Core {
    /// Abre (y migra si hace falta) las bases y construye la red.
    pub fn open(opts: CoreOptions) -> Result<Arc<Core>> {
        std::fs::create_dir_all(&opts.data_dir)?;
        let outcome = migrate::prepare(&opts.data_dir)?;
        let cache = db::open_cache(&opts.data_dir.join("cache.db"))?;
        let app = db::open_app(&opts.data_dir.join("app.db"))?;
        // Si la app se cerró durante una descarga, queda en pausa.
        app.execute(
            "UPDATE downloads SET state = 'paused' WHERE state IN ('active','queued')",
            [],
        )?;
        let settings = Settings::load(&app);
        let base_url = opts.base_url.unwrap_or_else(|| catalog::BASE.to_string());
        let net = Net::build(net_config(&settings, None, &base_url))?;
        std::fs::create_dir_all(
            settings
                .download_dir
                .clone()
                .unwrap_or_else(|| opts.data_dir.join("Downloads")),
        )?;

        Ok(Arc::new(Core {
            cache: Mutex::new(cache),
            app: Mutex::new(app),
            net: RwLock::new(Arc::new(net)),
            events: opts.events,
            wake: opts.wake,
            rt: opts.runtime,
            data_dir: opts.data_dir,
            base_url,
            cancels: Mutex::new(HashMap::new()),
            dl_sem: Arc::new(Semaphore::new(settings.max_active_downloads as usize)),
            crawl_stop: Arc::new(AtomicBool::new(false)),
            poster_sem: Arc::new(Semaphore::new(3)),
            proxy_password: Mutex::new(None),
            migration_notice: outcome.notice,
        }))
    }

    pub(crate) fn emit(&self, event: Event) {
        let _ = self.events.send(event); // si la UI ya no escucha, se descarta (E7)
        (self.wake)();
    }

    pub(crate) fn notice(&self, level: Level, message: impl Into<String>) {
        self.emit(Event::Notice(Notice {
            level,
            message: message.into(),
        }));
    }

    pub fn data_dir(&self) -> &Path {
        &self.data_dir
    }

    pub fn downloads_dir(&self) -> PathBuf {
        self.settings()
            .ok()
            .and_then(|s| s.download_dir)
            .unwrap_or_else(|| self.data_dir.join("Downloads"))
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    pub fn migration_notice(&self) -> Option<String> {
        self.migration_notice.clone()
    }

    // ---- Ajustes ---------------------------------------------------------------------

    pub fn settings(&self) -> Result<Settings> {
        Ok(Settings::load(&*lock(&self.app)?))
    }

    /// Valida, guarda y reconstruye la red; después relanza el diagnóstico.
    pub fn save_settings(self: &Arc<Self>, new: &Settings) -> Result<()> {
        new.validate()?;
        let old = self.settings()?;
        // Construir primero: si la red no es válida no se guarda nada.
        let password = lock(&self.proxy_password)?.clone();
        let net = Net::build(net_config(new, password, &self.base_url))?;
        if let Some(dir) = &new.download_dir {
            std::fs::create_dir_all(dir)?;
        }
        new.save(&*lock(&self.app)?)?;
        let network_changed = old.net_mode != new.net_mode
            || old.proxy != new.proxy
            || old.proxy_user != new.proxy_user
            || old.ftp_user != new.ftp_user;
        if network_changed {
            *self.net.write().map_err(|_| Error::Poisoned)? = Arc::new(net);
            self.run_probe();
        }
        Ok(())
    }

    /// Contraseña del proxy para esta sesión (no se persiste). Reconstruye la red.
    pub fn set_proxy_password(self: &Arc<Self>, password: Option<String>) -> Result<()> {
        *lock(&self.proxy_password)? = password.clone().filter(|p| !p.is_empty());
        let settings = self.settings()?;
        let net = Net::build(net_config(
            &settings,
            password.filter(|p| !p.is_empty()),
            &self.base_url,
        ))?;
        *self.net.write().map_err(|_| Error::Poisoned)? = Arc::new(net);
        self.run_probe();
        Ok(())
    }

    // ---- Red -------------------------------------------------------------------------

    pub fn net(&self) -> Arc<Net> {
        self.net
            .read()
            .map(|n| n.clone())
            .unwrap_or_else(|p| p.into_inner().clone())
    }

    pub fn active_mode(&self) -> Mode {
        self.net().active_mode()
    }

    /// Proxy efectivo con la contraseña enmascarada.
    pub fn proxy_label(&self) -> Option<String> {
        transport::probe::proxy_label(&self.net())
    }

    /// Lanza el diagnóstico; emite `Event::Probe` (uno por llamada).
    pub fn run_probe(self: &Arc<Self>) {
        let core = self.clone();
        self.rt.spawn(async move {
            let net = core.net();
            let report = transport::probe::run(&net).await;
            net.set_ftp_available(report.ftp_usable);
            core.emit(Event::Probe(report));
        });
    }
}

/// Construye la configuración de red desde los ajustes (y la contraseña de sesión).
pub(crate) fn net_config(
    settings: &Settings,
    proxy_password: Option<String>,
    base_url: &str,
) -> NetConfig {
    let mut cfg = NetConfig {
        mode: settings.net_mode,
        proxy: settings.proxy.clone(),
        base_url: base_url.to_string(),
        ..NetConfig::default()
    };
    cfg.ftp.user = settings.ftp_user.clone();
    // Con un catálogo alternativo (pruebas), el FTP apunta al mismo host.
    if base_url != catalog::BASE {
        if let Some(host) = url::Url::parse(base_url)
            .ok()
            .and_then(|u| u.host_str().map(str::to_string))
        {
            cfg.ftp.host = host;
        }
    }
    // Solo en compilaciones de depuración (quickstart §3.6): en producción el host está fijado.
    if let (true, Ok(addr)) = (cfg!(debug_assertions), std::env::var("NOVAFLIX_FTP_ADDR")) {
        if let Some((host, port)) = addr.rsplit_once(':') {
            if let Ok(port) = port.parse() {
                cfg.ftp.host = host.to_string();
                cfg.ftp.port = port;
            }
        }
    }
    if let ProxyChoice::Custom(url) = &mut cfg.proxy {
        if let (Ok(mut parsed), false) = (url::Url::parse(url), settings.proxy_user.is_empty()) {
            let _ = parsed.set_username(&settings.proxy_user);
            let _ = parsed.set_password(proxy_password.as_deref());
            *url = parsed.to_string();
        }
    }
    cfg
}
