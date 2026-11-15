//! Cliente FTP mínimo (solo lectura) sobre tokio: sesión de control, EPSV/PASV,
//! MLSD/LIST, SIZE y RETR con REST. Puede salir por un proxy SOCKS5.

use std::{
    net::{Ipv4Addr, SocketAddr},
    sync::OnceLock,
    time::Duration,
};

use bytes::Bytes;
use futures::{stream::BoxStream, StreamExt};
use regex::Regex;
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, AsyncReadExt, AsyncWrite, AsyncWriteExt, BufReader},
    net::TcpStream,
    time::timeout,
};

use super::Download;
use crate::{
    catalog::{self, RawEntry},
    error::{Error, Result},
};

const IO_TIMEOUT: Duration = Duration::from_secs(20);

#[derive(Debug, Clone)]
pub struct FtpConfig {
    pub host: String,
    pub port: u16,
    pub user: String,
    pub pass: String,
    /// `host:port` de un proxy SOCKS5 por el que salir (opcional).
    pub socks5: Option<String>,
}

impl Default for FtpConfig {
    fn default() -> Self {
        Self {
            host: catalog::HOST.to_string(),
            port: 21,
            user: "anonymous".to_string(),
            pass: "novaflix@".to_string(),
            socks5: None,
        }
    }
}

trait Io: AsyncRead + AsyncWrite + Unpin + Send {}
impl<T: AsyncRead + AsyncWrite + Unpin + Send> Io for T {}
type Conn = Box<dyn Io>;

async fn dial(cfg: &FtpConfig, host: &str, port: u16) -> Result<Conn> {
    let connect = async {
        match &cfg.socks5 {
            Some(proxy) => {
                let stream = tokio_socks::tcp::Socks5Stream::connect(proxy.as_str(), (host, port))
                    .await
                    .map_err(|e| Error::Ftp(format!("proxy SOCKS5: {e}")))?;
                Ok::<Conn, Error>(Box::new(stream))
            }
            None => {
                let stream = TcpStream::connect((host, port)).await?;
                stream.set_nodelay(true).ok();
                let boxed: Conn = Box::new(stream);
                Ok(boxed)
            }
        }
    };
    timeout(IO_TIMEOUT, connect)
        .await
        .map_err(|_| Error::Ftp(format!("tiempo agotado conectando a {host}:{port}")))?
}

pub struct Session {
    cfg: FtpConfig,
    control: BufReader<Conn>,
}

impl Session {
    pub async fn connect(cfg: &FtpConfig) -> Result<Self> {
        let conn = dial(cfg, &cfg.host, cfg.port).await?;
        let mut session = Session {
            cfg: cfg.clone(),
            control: BufReader::new(conn),
        };
        expect(session.reply().await?, &[220])?;

        let (code, text) = session.cmd(&format!("USER {}", cfg.user)).await?;
        match code {
            230 => {}
            331 | 332 => expect(session.cmd(&format!("PASS {}", cfg.pass)).await?, &[230])?,
            _ => return Err(Error::Ftp(format!("login rechazado: {code} {text}"))),
        }
        session.cmd("TYPE I").await?;
        // Mejor esfuerzo: nombres en UTF-8.
        session.cmd("OPTS UTF8 ON").await.ok();
        Ok(session)
    }

    async fn reply(&mut self) -> Result<(u16, String)> {
        let mut text = String::new();
        let mut code = 0_u16;
        loop {
            let mut line = String::new();
            let read = timeout(IO_TIMEOUT, self.control.read_line(&mut line))
                .await
                .map_err(|_| Error::Ftp("el servidor no responde".into()))??;
            if read == 0 {
                return Err(Error::Ftp("conexión cerrada por el servidor".into()));
            }
            let line = line.trim_end_matches(['\r', '\n']);
            text.push_str(line);
            text.push('\n');
            let bytes = line.as_bytes();
            if bytes.len() >= 4 && bytes[..3].iter().all(u8::is_ascii_digit) {
                let current: u16 = line[..3].parse().unwrap_or(0);
                if code == 0 {
                    code = current;
                }
                if current == code && bytes[3] == b' ' {
                    return Ok((code, text.trim().to_string()));
                }
            }
        }
    }

    async fn cmd(&mut self, command: &str) -> Result<(u16, String)> {
        let line = format!("{command}\r\n");
        timeout(
            IO_TIMEOUT,
            self.control.get_mut().write_all(line.as_bytes()),
        )
        .await
        .map_err(|_| Error::Ftp("el servidor no responde".into()))??;
        self.reply().await
    }

    /// Abre la conexión de datos (EPSV y, si no, PASV).
    async fn data_conn(&mut self) -> Result<Conn> {
        let (code, text) = self.cmd("EPSV").await?;
        if code == 229 {
            if let Some(port) = parse_epsv(&text) {
                return dial(&self.cfg.clone(), &self.cfg.host.clone(), port).await;
            }
        }
        let (code, text) = self.cmd("PASV").await?;
        expect((code, text.clone()), &[227])?;
        let (ip, port) = parse_pasv(&text)
            .ok_or_else(|| Error::Ftp(format!("respuesta PASV no válida: {text}")))?;
        // Los servidores tras NAT anuncian IPs internas: se usa el host de control salvo
        // que la IP anunciada sea pública y distinta.
        let host = if ip.is_private() || ip.is_loopback() || ip.is_unspecified() {
            self.cfg.host.clone()
        } else {
            ip.to_string()
        };
        dial(&self.cfg.clone(), &host, port).await
    }

    async fn cwd(&mut self, remote: &str) -> Result<()> {
        let (code, text) = self.cmd(&format!("CWD {remote}")).await?;
        if code == 250 {
            Ok(())
        } else {
            Err(Error::Ftp(format!(
                "no se puede abrir {remote}: {code} {text}"
            )))
        }
    }

    async fn transfer_text(&mut self, command: &str) -> Result<(u16, String)> {
        let mut data = self.data_conn().await?;
        let (code, text) = self.cmd(command).await?;
        if code != 125 && code != 150 {
            return Ok((code, text));
        }
        let mut buf = Vec::new();
        timeout(Duration::from_secs(120), data.read_to_end(&mut buf))
            .await
            .map_err(|_| Error::Ftp("tiempo agotado leyendo el listado".into()))??;
        drop(data);
        let _ = self.reply().await?;
        Ok((150, String::from_utf8_lossy(&buf).into_owned()))
    }

    pub async fn list(&mut self, path: &str) -> Result<Vec<RawEntry>> {
        let remote = remote_path(path);
        self.cwd(&remote).await?;

        let (code, body) = self.transfer_text("MLSD").await?;
        if code == 150 {
            return Ok(parse_mlsd(&body));
        }
        let (code, body) = self.transfer_text("LIST").await?;
        if code == 150 {
            return Ok(parse_list(&body));
        }
        Err(Error::Ftp(format!("el listado falló: {code} {body}")))
    }

    pub async fn size(&mut self, path: &str) -> Option<u64> {
        let (code, text) = self
            .cmd(&format!("SIZE {}", remote_path(path)))
            .await
            .ok()?;
        if code == 213 {
            text.split_whitespace().nth(1)?.parse().ok()
        } else {
            None
        }
    }

    pub async fn retrieve(mut self, path: &str, offset: u64) -> Result<Download> {
        let remote = remote_path(path);
        let total = self.size(path).await;
        let mut resumed = false;
        if offset > 0 {
            let (code, _) = self.cmd(&format!("REST {offset}")).await?;
            resumed = code == 350;
        }
        let data = self.data_conn().await?;
        let (code, text) = self.cmd(&format!("RETR {remote}")).await?;
        if code != 125 && code != 150 {
            return Err(Error::Ftp(format!("RETR rechazado: {code} {text}")));
        }

        // Algunos servidores aceptan `REST` (350) pero envían el fichero desde el principio.
        // Si conocemos el tamaño, un flujo más largo que `total - offset` delata el engaño.
        let limit = total.map(|t| t.saturating_sub(if resumed { offset } else { 0 }));
        let stream = futures::stream::unfold(
            (Some(data), self, vec![0_u8; 64 * 1024], 0_u64),
            move |(data, session, mut buf, seen)| async move {
                let mut data = data?;
                match timeout(IO_TIMEOUT, data.read(&mut buf)).await {
                    Err(_) => Some((
                        Err(Error::Ftp("la transferencia se quedó sin datos".into())),
                        (None, session, buf, seen),
                    )),
                    Ok(Err(err)) => Some((Err(err.into()), (None, session, buf, seen))),
                    Ok(Ok(0)) => None,
                    Ok(Ok(n)) => {
                        let seen = seen + n as u64;
                        if limit.is_some_and(|max| seen > max) {
                            return Some((
                                Err(Error::Ftp(
                                    "el servidor ignoró REST y envió el fichero desde el inicio"
                                        .into(),
                                )),
                                (None, session, buf, seen),
                            ));
                        }
                        let chunk = Bytes::copy_from_slice(&buf[..n]);
                        Some((Ok(chunk), (Some(data), session, buf, seen)))
                    }
                }
            },
        )
        .boxed();

        Ok(Download {
            total,
            resumed,
            stream: stream as BoxStream<'static, Result<Bytes>>,
        })
    }
}

fn expect(reply: (u16, String), codes: &[u16]) -> Result<()> {
    if codes.contains(&reply.0) {
        Ok(())
    } else {
        Err(Error::Ftp(format!("{} {}", reply.0, reply.1)))
    }
}

/// Convierte la ruta canónica (codificada como en las URL) a la ruta FTP real.
fn remote_path(path: &str) -> String {
    let decoded = catalog::decode_name(path);
    let trimmed = decoded.trim_end_matches('/');
    if trimmed.is_empty() {
        "/".to_string()
    } else if trimmed.starts_with('/') {
        trimmed.to_string()
    } else {
        format!("/{trimmed}")
    }
}

fn parse_epsv(text: &str) -> Option<u16> {
    let start = text.find("(|||")? + 4;
    let end = text[start..].find('|')? + start;
    text[start..end].parse().ok()
}

fn parse_pasv(text: &str) -> Option<(Ipv4Addr, u16)> {
    let start = text.find('(')? + 1;
    let end = text[start..].find(')')? + start;
    let nums: Vec<u16> = text[start..end]
        .split(',')
        .filter_map(|n| n.trim().parse().ok())
        .collect();
    if nums.len() != 6 {
        return None;
    }
    let ip = Ipv4Addr::new(nums[0] as u8, nums[1] as u8, nums[2] as u8, nums[3] as u8);
    Some((ip, nums[4] * 256 + nums[5]))
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

fn entry(name: &str, is_dir: bool, size: Option<i64>, mtime: Option<String>) -> RawEntry {
    let encoded = catalog::encode_name(name);
    RawEntry {
        href: if is_dir {
            format!("{encoded}/")
        } else {
            encoded
        },
        name: name.to_string(),
        is_dir,
        size,
        mtime,
    }
}

pub fn parse_mlsd(body: &str) -> Vec<RawEntry> {
    let mut out = Vec::new();
    for line in body.lines() {
        let Some((facts, name)) = line.split_once(' ') else {
            continue;
        };
        let mut kind = "";
        let mut size = None;
        let mut mtime = None;
        for fact in facts.split(';').filter(|f| !f.is_empty()) {
            let Some((key, value)) = fact.split_once('=') else {
                continue;
            };
            match key.to_ascii_lowercase().as_str() {
                "type" => kind = value,
                "size" => size = value.parse::<i64>().ok(),
                "modify" if value.len() >= 12 => {
                    mtime = Some(format!(
                        "{}-{}-{} {}:{}",
                        &value[0..4],
                        &value[4..6],
                        &value[6..8],
                        &value[8..10],
                        &value[10..12]
                    ));
                }
                _ => {}
            }
        }
        match kind.to_ascii_lowercase().as_str() {
            "dir" => out.push(entry(name, true, None, mtime)),
            "file" => out.push(entry(name, false, size, mtime)),
            _ => {}
        }
    }
    out
}

fn list_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"^([-d])[rwxsStTl-]{9}\s+\d+\s+\S+\s+\S+\s+(\d+)\s+([A-Za-z]{3})\s+(\d{1,2})\s+(\d{4}|\d{1,2}:\d{2})\s+(.+)$",
        )
        .expect("regex LIST inválida")
    })
}

pub fn parse_list(body: &str) -> Vec<RawEntry> {
    let this_year = chrono::Utc::now().format("%Y").to_string();
    let mut out = Vec::new();
    for line in body.lines() {
        let Some(c) = list_re().captures(line.trim_end()) else {
            continue;
        };
        let is_dir = &c[1] == "d";
        let name = c[6].to_string();
        if name == "." || name == ".." {
            continue;
        }
        let (year, time) = if c[5].contains(':') {
            (this_year.clone(), c[5].to_string())
        } else {
            (c[5].to_string(), "00:00".to_string())
        };
        let month = MONTHS
            .iter()
            .position(|m| m.eq_ignore_ascii_case(&c[3]))
            .map_or(1, |i| i + 1);
        let mtime = Some(format!("{year}-{month:02}-{:0>2} {:0>5}", &c[4], time));
        let size = if is_dir { None } else { c[2].parse().ok() };
        out.push(entry(&name, is_dir, size, mtime));
    }
    out
}

/// Abre una sesión, lista el directorio y la cierra.
pub async fn list_dir(cfg: &FtpConfig, path: &str) -> Result<Vec<RawEntry>> {
    let mut session = Session::connect(cfg).await?;
    let entries = session.list(path).await;
    let _ = session.cmd("QUIT").await;
    entries
}

pub async fn open(cfg: &FtpConfig, path: &str, offset: u64) -> Result<Download> {
    Session::connect(cfg).await?.retrieve(path, offset).await
}

/// Comprueba el puerto de control y devuelve el saludo del servidor.
pub async fn greeting(addr: SocketAddr, wait: Duration) -> Result<String> {
    let stream = timeout(wait, TcpStream::connect(addr))
        .await
        .map_err(|_| Error::Ftp("tiempo agotado".into()))??;
    let mut reader = BufReader::new(stream);
    let mut line = String::new();
    timeout(wait, reader.read_line(&mut line))
        .await
        .map_err(|_| Error::Ftp("sin saludo del servidor".into()))??;
    Ok(line.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mlsd_lines() {
        let body = "type=dir;modify=20260929053000; Peliculas\r\ntype=file;size=637;modify=20240207101500; REDES_SOCIALES.txt\ntype=cdir; .\n";
        let entries = parse_mlsd(body);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_dir);
        assert_eq!(entries[0].href, "Peliculas/");
        assert_eq!(entries[0].mtime.as_deref(), Some("2026-09-29 05:30"));
        assert_eq!(entries[1].size, Some(637));
    }

    #[test]
    fn unix_list_lines() {
        let body = "drwxr-xr-x   2 ftp ftp      4096 Sep 29 05:30 Series B\n-rw-r--r--   1 ftp ftp   3221225472 Jan 02  2025 Wicked.mp4\n";
        let entries = parse_list(body);
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].href, "Series%20B/");
        assert_eq!(entries[1].size, Some(3_221_225_472));
        assert_eq!(entries[1].mtime.as_deref(), Some("2025-01-02 00:00"));
    }

    #[test]
    fn passive_replies() {
        assert_eq!(
            parse_epsv("229 Entering Extended Passive Mode (|||40021|)"),
            Some(40021)
        );
        assert_eq!(
            parse_pasv("227 Entering Passive Mode (10,0,0,5,156,64)"),
            Some((Ipv4Addr::new(10, 0, 0, 5), 40000))
        );
    }

    #[test]
    fn remote_paths() {
        assert_eq!(
            remote_path("/Peliculas/Wicked%20%282024%29/"),
            "/Peliculas/Wicked (2024)"
        );
        assert_eq!(remote_path("/"), "/");
    }
}
