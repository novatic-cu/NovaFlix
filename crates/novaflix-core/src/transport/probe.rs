//! Diagnóstico de red: qué protocolos funcionan desde este equipo y por qué.

use std::{
    net::SocketAddr,
    time::{Duration, Instant},
};

use base64::Engine;
use serde::Serialize;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::timeout,
};

use super::{ftp, Mode, Net, ProxyChoice};

const STEP_TIMEOUT: Duration = Duration::from_secs(6);

#[derive(Debug, Clone, Serialize)]
pub struct Step {
    pub ok: bool,
    pub ms: u64,
    pub detail: String,
}

impl Step {
    fn new(ok: bool, started: Instant, detail: impl Into<String>) -> Self {
        Self {
            ok,
            ms: started.elapsed().as_millis() as u64,
            detail: detail.into(),
        }
    }

    pub fn skipped(detail: impl Into<String>) -> Self {
        Self {
            ok: false,
            ms: 0,
            detail: detail.into(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Report {
    /// Proxy detectado con la contraseña ocultada.
    pub proxy: Option<String>,
    pub dns: Step,
    pub ftp: Step,
    /// Túnel CONNECT al puerto 21 a través del proxy HTTP, si hay uno.
    pub ftp_via_proxy: Option<Step>,
    pub https: Step,
    pub ftp_usable: bool,
    pub http_usable: bool,
    pub recommendation: Mode,
    pub summary: String,
}

#[derive(Debug, Clone)]
struct ProxyEndpoint {
    scheme: String,
    host: String,
    port: u16,
    user: Option<String>,
    pass: Option<String>,
}

impl ProxyEndpoint {
    fn masked(&self) -> String {
        match &self.user {
            Some(user) => format!("{}://{}:***@{}:{}", self.scheme, user, self.host, self.port),
            None => format!("{}://{}:{}", self.scheme, self.host, self.port),
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        let raw = if raw.contains("://") {
            raw.to_string()
        } else {
            format!("http://{raw}")
        };
        let url = url::Url::parse(&raw).ok()?;
        let user = (!url.username().is_empty()).then(|| {
            percent_encoding::percent_decode_str(url.username())
                .decode_utf8_lossy()
                .into_owned()
        });
        let pass = url.password().map(|p| {
            percent_encoding::percent_decode_str(p)
                .decode_utf8_lossy()
                .into_owned()
        });
        Some(Self {
            scheme: url.scheme().to_string(),
            host: url.host_str()?.to_string(),
            port: url.port_or_known_default().unwrap_or(3128),
            user,
            pass,
        })
    }
}

/// Proxy que usaría el sistema (variables de entorno estándar).
fn system_proxy() -> Option<ProxyEndpoint> {
    [
        "HTTPS_PROXY",
        "https_proxy",
        "ALL_PROXY",
        "all_proxy",
        "HTTP_PROXY",
        "http_proxy",
    ]
    .iter()
    .filter_map(|key| std::env::var(key).ok())
    .find(|value| !value.trim().is_empty())
    .and_then(|value| ProxyEndpoint::parse(value.trim()))
}

pub fn proxy_label(net: &Net) -> Option<String> {
    effective_proxy(net).map(|p| p.masked())
}

fn effective_proxy(net: &Net) -> Option<ProxyEndpoint> {
    match &net.config().proxy {
        ProxyChoice::Direct => None,
        ProxyChoice::Custom(url) => ProxyEndpoint::parse(url),
        ProxyChoice::System => system_proxy(),
    }
}

async fn connect_through_proxy(proxy: &ProxyEndpoint, target: &str) -> Step {
    let started = Instant::now();
    let attempt = async {
        let stream = TcpStream::connect((proxy.host.as_str(), proxy.port))
            .await
            .map_err(|e| format!("no se pudo contactar al proxy: {e}"))?;
        let mut reader = BufReader::new(stream);
        let mut request = format!("CONNECT {target} HTTP/1.1\r\nHost: {target}\r\n");
        if let (Some(user), Some(pass)) = (&proxy.user, &proxy.pass) {
            let token = base64::engine::general_purpose::STANDARD.encode(format!("{user}:{pass}"));
            request.push_str(&format!("Proxy-Authorization: Basic {token}\r\n"));
        }
        request.push_str("\r\n");
        reader
            .get_mut()
            .write_all(request.as_bytes())
            .await
            .map_err(|e| e.to_string())?;
        let mut status = String::new();
        reader
            .read_line(&mut status)
            .await
            .map_err(|e| e.to_string())?;
        Ok::<String, String>(status.trim().to_string())
    };
    match timeout(STEP_TIMEOUT, attempt).await {
        Err(_) => Step::new(false, started, "el proxy no respondió"),
        Ok(Err(err)) => Step::new(false, started, err),
        Ok(Ok(status)) => {
            let ok = status.contains(" 200");
            let detail = if ok {
                "túnel permitido".to_string()
            } else {
                format!("el proxy respondió «{status}»")
            };
            Step::new(ok, started, detail)
        }
    }
}

/// El HTTPS por un proxy nacional puede tardar en la primera conexión: más margen y un reintento,
/// para no declarar «sin conexión» por un pico de latencia.
const HTTPS_TIMEOUT: Duration = Duration::from_secs(12);

async fn https_step(net: &Net) -> Step {
    let started = Instant::now();
    let mut last = String::new();
    for _ in 0..2 {
        match net
            .http()
            .head(format!("{}/", net.base_url().trim_end_matches('/')))
            .timeout(HTTPS_TIMEOUT)
            .send()
            .await
        {
            Ok(response) if response.status().is_success() => {
                return Step::new(
                    true,
                    started,
                    format!("HTTP {}", response.status().as_u16()),
                );
            }
            Ok(response) => last = format!("HTTP {}", response.status()),
            Err(err) if err.is_timeout() => last = "tiempo agotado".to_string(),
            Err(err) if err.is_connect() => last = "no se pudo conectar".to_string(),
            Err(err) => last = err.without_url().to_string(),
        }
    }
    Step::new(false, started, last)
}

/// DNS directo y, con su resultado, el saludo del puerto 21. En redes con proxy el DNS directo
/// suele fallar: es normal y no impide usar HTTP.
async fn ftp_steps(net: &Net, dns_wait: Duration) -> (Step, Step) {
    let cfg = &net.config().ftp;
    let started = Instant::now();
    let dns = match timeout(
        dns_wait,
        tokio::net::lookup_host((cfg.host.as_str(), cfg.port)),
    )
    .await
    {
        Ok(Ok(mut addrs)) => match addrs.next() {
            Some(addr) => (Step::new(true, started, addr.ip().to_string()), Some(addr)),
            None => (Step::new(false, started, "sin direcciones"), None),
        },
        Ok(Err(err)) => (Step::new(false, started, err.to_string()), None),
        Err(_) => (Step::new(false, started, "tiempo agotado"), None),
    };

    let started = Instant::now();
    let ftp_step = if cfg.socks5.is_some() {
        match ftp::Session::connect(cfg).await {
            Ok(_) => Step::new(true, started, "FTP por proxy SOCKS5"),
            Err(err) => Step::new(false, started, err.to_string()),
        }
    } else if let Some(addr) = dns.1 {
        let addr: SocketAddr = addr;
        match ftp::greeting(addr, STEP_TIMEOUT).await {
            Ok(hello) => Step::new(hello.starts_with("220"), started, hello),
            Err(err) => Step::new(false, started, err.to_string()),
        }
    } else {
        Step::skipped("sin DNS directo")
    };
    (dns.0, ftp_step)
}

pub async fn run(net: &Net) -> Report {
    let ftp_cfg = &net.config().ftp;
    let proxy = effective_proxy(net);

    // Con proxy, el DNS directo casi siempre falla: no merece esperar tanto.
    let dns_wait = if proxy.is_some() {
        Duration::from_secs(3)
    } else {
        STEP_TIMEOUT
    };

    // Los tres grupos de pruebas son independientes: se lanzan a la vez.
    let tunnel = async {
        match &proxy {
            Some(p) if p.scheme.starts_with("http") && ftp_cfg.socks5.is_none() => {
                Some(connect_through_proxy(p, &format!("{}:{}", ftp_cfg.host, ftp_cfg.port)).await)
            }
            _ => None,
        }
    };
    let ((dns, ftp_step), ftp_via_proxy, https) =
        tokio::join!(ftp_steps(net, dns_wait), tunnel, https_step(net));

    let ftp_usable = ftp_step.ok;
    let http_usable = https.ok;
    let recommendation = if ftp_usable { Mode::Ftp } else { Mode::Http };

    let summary = match (ftp_usable, http_usable) {
        (true, true) => "FTP y HTTP funcionan desde aquí.".to_string(),
        (true, false) => "FTP funciona, pero HTTP no responde.".to_string(),
        (false, true) => match &ftp_via_proxy {
            Some(step) if !step.ok => format!(
                "Tu proxy no deja pasar FTP ({}). NovaFlix usará HTTP, que sí funciona.",
                step.detail
            ),
            _ => "FTP no es accesible desde esta red. NovaFlix usará HTTP, que sí funciona."
                .to_string(),
        },
        (false, false) => {
            "No hay conexión con el servidor. Revisa la red o la configuración del proxy."
                .to_string()
        }
    };

    Report {
        proxy: proxy.map(|p| p.masked()),
        dns,
        ftp: ftp_step,
        ftp_via_proxy,
        https,
        ftp_usable,
        http_usable,
        recommendation,
        summary,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_credentials_are_masked() {
        let proxy = ProxyEndpoint::parse("http://ana:secreto@10.0.0.1:3128").unwrap();
        assert_eq!(proxy.pass.as_deref(), Some("secreto"));
        assert_eq!(proxy.masked(), "http://ana:***@10.0.0.1:3128");
        assert!(!proxy.masked().contains("secreto"));
    }

    #[test]
    fn proxy_without_scheme() {
        let proxy = ProxyEndpoint::parse("10.0.0.1:3128").unwrap();
        assert_eq!(proxy.scheme, "http");
        assert_eq!(proxy.port, 3128);
    }
}
