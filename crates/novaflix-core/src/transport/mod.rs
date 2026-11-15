//! Transporte hacia el servidor: HTTP(S) o FTP, a elección del usuario.
//!
//! Las rutas del catálogo son canónicas (codificadas como en las URL de Apache);
//! cada transporte las traduce a lo que necesita.

use std::{
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    time::Duration,
};

use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt};
use serde::{Deserialize, Serialize};

use crate::{
    catalog::{self, RawEntry},
    error::{Error, Result},
};

pub mod ftp;
pub mod http;
pub mod probe;

pub use ftp::FtpConfig;

pub const USER_AGENT: &str = concat!("NovaFlix/", env!("CARGO_PKG_VERSION"));

/// Protocolo elegido por el usuario.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum Mode {
    /// FTP si el diagnóstico dice que es posible; HTTP en caso contrario.
    #[default]
    Auto,
    Http,
    Ftp,
}

impl Mode {
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Auto => "auto",
            Mode::Http => "http",
            Mode::Ftp => "ftp",
        }
    }

    pub fn parse(value: &str) -> Self {
        match value {
            "http" => Mode::Http,
            "ftp" => Mode::Ftp,
            _ => Mode::Auto,
        }
    }
}

/// Cómo salir a la red.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ProxyChoice {
    /// Variables de entorno (`HTTPS_PROXY`...) o proxy del sistema en Windows.
    #[default]
    System,
    /// Conexión directa, ignorando cualquier proxy.
    Direct,
    /// Proxy propio: `http://host:puerto`, `https://...` o `socks5://...`.
    Custom(String),
}

#[derive(Debug, Clone)]
pub struct NetConfig {
    pub mode: Mode,
    pub proxy: ProxyChoice,
    pub ftp: FtpConfig,
    /// Raíz del catálogo. Solo cambia en pruebas; en producción es `catalog::BASE`.
    pub base_url: String,
}

impl Default for NetConfig {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            proxy: ProxyChoice::default(),
            ftp: FtpConfig::default(),
            base_url: catalog::BASE.to_string(),
        }
    }
}

pub struct Download {
    /// Tamaño total del fichero si el servidor lo informa.
    pub total: Option<u64>,
    /// `true` si el servidor aceptó continuar desde el desplazamiento pedido.
    pub resumed: bool,
    pub stream: BoxStream<'static, Result<Bytes>>,
}

pub struct Net {
    http: reqwest::Client,
    cfg: NetConfig,
    /// Resultado del último diagnóstico: ¿se puede usar FTP desde aquí?
    ftp_ok: Arc<AtomicBool>,
}

impl Net {
    pub fn build(mut cfg: NetConfig) -> Result<Self> {
        let mut builder = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(45))
            .redirect(same_host_redirects(&cfg.base_url));

        match &cfg.proxy {
            ProxyChoice::System => {}
            ProxyChoice::Direct => builder = builder.no_proxy(),
            ProxyChoice::Custom(url) => {
                let proxy = reqwest::Proxy::all(url.as_str())
                    .map_err(|e| Error::Net(format!("proxy no válido: {e}")))?;
                builder = builder.no_proxy().proxy(proxy);
                // FTP solo sabe salir por SOCKS5; un proxy HTTP no sirve para FTP.
                cfg.ftp.socks5 = socks5_endpoint(url);
            }
        }

        Ok(Self {
            http: builder.build()?,
            cfg,
            ftp_ok: Arc::new(AtomicBool::new(false)),
        })
    }

    pub fn config(&self) -> &NetConfig {
        &self.cfg
    }

    pub fn base_url(&self) -> &str {
        &self.cfg.base_url
    }

    pub fn http(&self) -> &reqwest::Client {
        &self.http
    }

    pub fn set_ftp_available(&self, available: bool) {
        self.ftp_ok.store(available, Ordering::Relaxed);
    }

    /// Protocolo que se usará ahora mismo.
    pub fn active_mode(&self) -> Mode {
        match self.cfg.mode {
            Mode::Auto if self.ftp_ok.load(Ordering::Relaxed) => Mode::Ftp,
            Mode::Auto => Mode::Http,
            other => other,
        }
    }

    pub async fn list_dir(&self, path: &str) -> Result<Vec<RawEntry>> {
        match self.active_mode() {
            Mode::Ftp => match ftp::list_dir(&self.cfg.ftp, path).await {
                Ok(entries) => Ok(entries),
                // En automático, un fallo de FTP no debe dejar al usuario sin catálogo.
                Err(_) if self.cfg.mode == Mode::Auto => {
                    http::list(&self.http, &self.cfg.base_url, path).await
                }
                Err(err) => Err(err),
            },
            _ => http::list(&self.http, &self.cfg.base_url, path).await,
        }
    }

    pub async fn open(&self, path: &str, offset: u64) -> Result<Download> {
        match self.active_mode() {
            Mode::Ftp => match ftp::open(&self.cfg.ftp, path, offset).await {
                Ok(download) => Ok(download),
                Err(_) if self.cfg.mode == Mode::Auto => {
                    http::open(&self.http, &self.cfg.base_url, path, offset).await
                }
                Err(err) => Err(err),
            },
            _ => http::open(&self.http, &self.cfg.base_url, path, offset).await,
        }
    }

    /// `listado.html`: el árbol completo que publica el sitio (siempre por HTTP).
    pub async fn fetch_listado(&self) -> Result<String> {
        http::fetch_text(
            &self.http,
            &format!("{}/listado.html", self.cfg.base_url.trim_end_matches('/')),
        )
        .await
    }
}

/// Solo se siguen redirecciones al mismo host del catálogo (Constitución VI, FR-017).
fn same_host_redirects(base_url: &str) -> reqwest::redirect::Policy {
    let allowed = url::Url::parse(base_url)
        .ok()
        .and_then(|u| u.host_str().map(str::to_string))
        .unwrap_or_else(|| catalog::HOST.to_string());
    reqwest::redirect::Policy::custom(move |attempt| {
        if attempt.previous().len() >= 5 {
            attempt.error("demasiadas redirecciones")
        } else if attempt.url().host_str() == Some(allowed.as_str()) {
            attempt.follow()
        } else {
            attempt.error("redirección a otro servidor rechazada")
        }
    })
}

/// Extrae `host:puerto` de una URL `socks5://` / `socks5h://`.
fn socks5_endpoint(url: &str) -> Option<String> {
    let parsed = url::Url::parse(url).ok()?;
    if !parsed.scheme().starts_with("socks5") {
        return None;
    }
    Some(format!(
        "{}:{}",
        parsed.host_str()?,
        parsed.port().unwrap_or(1080)
    ))
}

/// Une los trozos de un flujo en memoria (útil en pruebas).
pub async fn collect(mut download: Download) -> Result<Vec<u8>> {
    let mut out = Vec::new();
    while let Some(chunk) = download.stream.next().await {
        out.extend_from_slice(&chunk?);
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mode_roundtrip() {
        for mode in [Mode::Auto, Mode::Http, Mode::Ftp] {
            assert_eq!(Mode::parse(mode.as_str()), mode);
        }
        assert_eq!(Mode::parse("???"), Mode::Auto);
    }

    #[test]
    fn socks_endpoint() {
        assert_eq!(
            socks5_endpoint("socks5://10.0.0.1:9050").as_deref(),
            Some("10.0.0.1:9050")
        );
        assert_eq!(socks5_endpoint("http://10.0.0.1:3128"), None);
    }

    #[test]
    fn auto_follows_probe() {
        let net = Net::build(NetConfig::default()).unwrap();
        assert_eq!(net.active_mode(), Mode::Http);
        net.set_ftp_available(true);
        assert_eq!(net.active_mode(), Mode::Ftp);
    }
}
