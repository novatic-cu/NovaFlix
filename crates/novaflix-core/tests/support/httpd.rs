//! Servidor HTTP mínimo que imita el índice de Apache (`mod_autoindex`) para pruebas.

use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};

use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    net::{TcpListener, TcpStream},
};

pub const MOVIE_LEN: usize = 300_000;

pub fn movie_bytes() -> Vec<u8> {
    (0..MOVIE_LEN as u32).map(|i| (i % 253) as u8).collect()
}

pub struct Httpd {
    pub base: String,
    pub hits: Arc<AtomicUsize>,
}

pub async fn spawn() -> Httpd {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let hits = Arc::new(AtomicUsize::new(0));
    let counter = hits.clone();
    tokio::spawn(async move {
        loop {
            let Ok((stream, _)) = listener.accept().await else {
                break;
            };
            let counter = counter.clone();
            tokio::spawn(async move {
                let _ = handle(stream, counter).await;
            });
        }
    });
    Httpd { base, hits }
}

const LISTADO: &str = r#"<html><body>
<a href="http://visuales.uclv.cu//Peliculas/">Peliculas</a>
<a href="http://visuales.uclv.cu//Peliculas/Wicked%20%282024%29/">Wicked (2024)</a>
<a href="http://visuales.uclv.cu//Series/">Series</a>
</body></html>"#;

/// Formato real del servidor (Apache con tabla y fechas ISO), capturado el 2026-11-14.
fn row(href: &str, label: &str, date: &str, size: &str) -> String {
    format!(
        "<tr><td valign=\"top\"><img src=\"/icons/x.gif\"></td><td><a href=\"{href}\">{label}</a></td>\
         <td align=\"right\">{date}  </td><td align=\"right\">{size}</td><td>&nbsp;</td></tr>\n"
    )
}

fn page(rows: &[String]) -> String {
    format!(
        "<html><body><table>\n<tr><th><a href=\"?C=N;O=D\">Name</a></th></tr>\n\
         <tr><td><a href=\"/\">Parent Directory</a></td><td>&nbsp;</td><td align=\"right\">  - </td></tr>\n{}</table></body></html>",
        rows.concat()
    )
}

fn root_page() -> String {
    page(&[
        row("Peliculas/", "Peliculas/", "2026-09-29 05:30", "  - "),
        row("Series/", "Series/", "2026-09-29 05:30", "  - "),
        row("REDES.txt", "REDES.txt", "2024-02-07 10:15", "637"),
    ])
}

fn peliculas_page() -> String {
    page(&[
        row(
            "Wicked%20%282024%29/",
            "Wicked (2024)/",
            "2026-09-29 05:30",
            "  - ",
        ),
        row("Wicked.mp4", "Wicked.mp4", "2025-01-02 10:00", "293K"),
    ])
}

async fn respond(
    stream: &mut BufReader<TcpStream>,
    status: &str,
    headers: &[(&str, String)],
    body: &[u8],
) -> std::io::Result<()> {
    let mut head = format!(
        "HTTP/1.1 {status}\r\nConnection: close\r\nContent-Length: {}\r\n",
        body.len()
    );
    for (k, v) in headers {
        head.push_str(&format!("{k}: {v}\r\n"));
    }
    head.push_str("\r\n");
    stream.get_mut().write_all(head.as_bytes()).await?;
    stream.get_mut().write_all(body).await?;
    stream.get_mut().shutdown().await
}

async fn handle(stream: TcpStream, hits: Arc<AtomicUsize>) -> std::io::Result<()> {
    let mut io = BufReader::new(stream);
    let mut request_line = String::new();
    io.read_line(&mut request_line).await?;
    let mut range: Option<usize> = None;
    loop {
        let mut line = String::new();
        io.read_line(&mut line).await?;
        if line.trim().is_empty() {
            break;
        }
        if let Some(value) = line.to_ascii_lowercase().strip_prefix("range: bytes=") {
            range = value.trim().trim_end_matches('-').parse().ok();
        }
    }
    hits.fetch_add(1, Ordering::Relaxed);
    let path = request_line
        .split_whitespace()
        .nth(1)
        .unwrap_or("/")
        .to_string();
    let movie = movie_bytes();

    match path.as_str() {
        "/" => respond(&mut io, "200 OK", &[], root_page().as_bytes()).await,
        "/Peliculas/" => respond(&mut io, "200 OK", &[], peliculas_page().as_bytes()).await,
        "/listado.html" => respond(&mut io, "200 OK", &[], LISTADO.as_bytes()).await,
        "/Peliculas/Wicked.mp4" => match range {
            Some(start) if start < movie.len() => {
                let tail = &movie[start..];
                let content_range = format!("bytes {start}-{}/{}", movie.len() - 1, movie.len());
                respond(
                    &mut io,
                    "206 Partial Content",
                    &[("Content-Range", content_range)],
                    tail,
                )
                .await
            }
            _ => respond(&mut io, "200 OK", &[], &movie).await,
        },
        // Ignora `Range` y devuelve siempre el fichero entero.
        "/norange.bin" => respond(&mut io, "200 OK", &[], &movie).await,
        "/redir" => {
            respond(
                &mut io,
                "302 Found",
                &[("Location", "http://elsewhere.invalid/x".into())],
                b"",
            )
            .await
        }
        "/proxyauth" => respond(&mut io, "407 Proxy Authentication Required", &[], b"").await,
        _ => respond(&mut io, "404 Not Found", &[], b"nope").await,
    }
}
