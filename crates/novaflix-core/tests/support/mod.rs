//! Servidores de prueba (FTP y HTTP) para las pruebas de integración.
#![allow(dead_code)]

pub mod httpd;

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

#[derive(Clone)]
pub struct FtpdOptions {
    pub mlsd: bool,
    pub require_login: Option<(String, String)>,
}

impl Default for FtpdOptions {
    fn default() -> Self {
        Self {
            mlsd: true,
            require_login: None,
        }
    }
}

pub async fn spawn(root: PathBuf, opts: FtpdOptions) -> u16 {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let port = listener.local_addr().unwrap().port();
    let root = Arc::new(root);
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let root = root.clone();
            let opts = opts.clone();
            tokio::spawn(async move {
                let _ = handle(stream, root, opts).await;
            });
        }
    });
    port
}

async fn send(w: &mut BufReader<TcpStream>, line: &str) -> std::io::Result<()> {
    w.get_mut()
        .write_all(format!("{line}\r\n").as_bytes())
        .await
}

fn resolve(root: &Path, cwd: &str, arg: &str) -> PathBuf {
    let joined = if arg.starts_with('/') {
        arg.to_string()
    } else {
        format!("{}/{}", cwd.trim_end_matches('/'), arg)
    };
    let mut out = root.to_path_buf();
    for part in joined.split('/').filter(|p| !p.is_empty() && *p != "..") {
        out.push(part);
    }
    out
}

async fn handle(stream: TcpStream, root: Arc<PathBuf>, opts: FtpdOptions) -> std::io::Result<()> {
    let mut io = BufReader::new(stream);
    send(&mut io, "220 novaflix test ftpd").await?;
    let mut cwd = String::from("/");
    let mut data: Option<TcpListener> = None;
    let mut rest: u64 = 0;
    let mut user = String::new();

    loop {
        let mut line = String::new();
        if io.read_line(&mut line).await? == 0 {
            return Ok(());
        }
        let line = line.trim_end().to_string();
        let (cmd, arg) = line.split_once(' ').unwrap_or((line.as_str(), ""));
        match cmd.to_ascii_uppercase().as_str() {
            "USER" => {
                user = arg.to_string();
                send(&mut io, "331 password please").await?
            }
            "PASS" => match &opts.require_login {
                Some((u, p)) if &user != u || arg != p => {
                    send(&mut io, "530 login incorrect").await?
                }
                _ => send(&mut io, "230 ok").await?,
            },
            "TYPE" | "OPTS" => send(&mut io, "200 ok").await?,
            "EPSV" => {
                let l = TcpListener::bind("127.0.0.1:0").await?;
                let p = l.local_addr()?.port();
                data = Some(l);
                send(
                    &mut io,
                    &format!("229 Entering Extended Passive Mode (|||{p}|)"),
                )
                .await?;
            }
            "CWD" => {
                let target = resolve(&root, &cwd, arg);
                if target.is_dir() {
                    cwd = if arg.starts_with('/') {
                        arg.to_string()
                    } else {
                        format!("{}/{}", cwd.trim_end_matches('/'), arg)
                    };
                    send(&mut io, "250 ok").await?
                } else {
                    send(&mut io, "550 no such directory").await?
                }
            }
            "MLSD" | "LIST" => {
                if cmd.eq_ignore_ascii_case("MLSD") && !opts.mlsd {
                    send(&mut io, "500 MLSD not understood").await?;
                    continue;
                }
                let Some(l) = data.take() else {
                    send(&mut io, "425 use EPSV first").await?;
                    continue;
                };
                send(&mut io, "150 here it comes").await?;
                let (mut d, _) = l.accept().await?;
                let dir = resolve(&root, &cwd, "");
                let mut body = String::new();
                let mut entries: Vec<_> = std::fs::read_dir(dir)?.flatten().collect();
                entries.sort_by_key(|e| e.file_name());
                for e in entries {
                    let meta = e.metadata()?;
                    let name = e.file_name().to_string_lossy().into_owned();
                    if cmd.eq_ignore_ascii_case("MLSD") {
                        let kind = if meta.is_dir() { "dir" } else { "file" };
                        body.push_str(&format!(
                            "type={kind};size={};modify=20260929053000; {name}\r\n",
                            meta.len()
                        ));
                    } else {
                        let flag = if meta.is_dir() { 'd' } else { '-' };
                        body.push_str(&format!(
                            "{flag}rw-r--r--   1 ftp ftp {:>12} Sep 29 05:30 {name}\r\n",
                            meta.len()
                        ));
                    }
                }
                d.write_all(body.as_bytes()).await?;
                drop(d);
                send(&mut io, "226 done").await?;
            }
            "SIZE" => {
                let path = resolve(&root, &cwd, arg);
                match std::fs::metadata(path) {
                    Ok(m) if m.is_file() => send(&mut io, &format!("213 {}", m.len())).await?,
                    _ => send(&mut io, "550 no such file").await?,
                }
            }
            "REST" => {
                rest = arg.parse().unwrap_or(0);
                send(&mut io, "350 restarting").await?
            }
            "RETR" => {
                let path = resolve(&root, &cwd, arg);
                let Some(l) = data.take() else {
                    send(&mut io, "425 use EPSV first").await?;
                    continue;
                };
                match std::fs::read(path) {
                    Ok(bytes) => {
                        send(&mut io, "150 sending").await?;
                        let (mut d, _) = l.accept().await?;
                        d.write_all(&bytes[(rest as usize).min(bytes.len())..])
                            .await?;
                        drop(d);
                        rest = 0;
                        send(&mut io, "226 done").await?;
                    }
                    Err(_) => send(&mut io, "550 no such file").await?,
                }
            }
            "QUIT" => {
                send(&mut io, "221 bye").await?;
                return Ok(());
            }
            _ => send(&mut io, "502 not implemented").await?,
        }
    }
}
