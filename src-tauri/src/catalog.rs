use std::sync::OnceLock;

use regex::Regex;

pub const BASE: &str = "https://visuales.uclv.cu";

#[derive(Debug, Clone, serde::Serialize)]
pub struct RawEntry {
    pub href: String,
    pub name: String,
    pub is_dir: bool,
    pub size: Option<i64>,
    pub mtime: Option<String>,
}

fn row_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"<a href="([^"]+)">([^<]*)</a>\s*(\d{2}-[A-Za-z]{3}-\d{4} \d{2}:\d{2})\s+([^\s<]+)"#,
        )
        .expect("regex de filas invalido")
    })
}

fn listado_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r#"href="https?://visuales\.uclv\.cu//([^"]+)""#)
            .expect("regex de listado invalido")
    })
}

/// Decodifica percent-encoding con fallback latin-1 (el servidor sirve nombres mixtos).
pub fn decode_name(input: &str) -> String {
    let bytes: Vec<u8> = percent_encoding::percent_decode_str(input).collect();
    match String::from_utf8(bytes) {
        Ok(value) => value,
        Err(err) => err.into_bytes().iter().map(|byte| *byte as char).collect(),
    }
}

pub fn absolute_url(path: &str) -> String {
    let normalized = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };
    format!("{}{}", BASE, normalized.replace(' ', "%20"))
}

pub fn parse_size(raw: &str) -> Option<i64> {
    let raw = raw.trim();
    if raw.is_empty() || raw == "-" {
        return None;
    }
    let (number, multiplier) = match raw.chars().last() {
        Some('K') => (&raw[..raw.len() - 1], 1024_f64),
        Some('M') => (&raw[..raw.len() - 1], 1024_f64.powi(2)),
        Some('G') => (&raw[..raw.len() - 1], 1024_f64.powi(3)),
        Some('T') => (&raw[..raw.len() - 1], 1024_f64.powi(4)),
        _ => (raw, 1_f64),
    };
    number
        .parse::<f64>()
        .ok()
        .map(|value| (value * multiplier) as i64)
}

pub fn parse_listing(html: &str) -> Vec<RawEntry> {
    let mut entries = Vec::new();
    for captures in row_re().captures_iter(html) {
        let href = captures.get(1).map(|m| m.as_str()).unwrap_or_default();
        if href == "../" || href.starts_with('?') || href.starts_with("http") {
            continue;
        }
        let is_dir = href.ends_with('/');
        let name = decode_name(href.trim_end_matches('/'));
        entries.push(RawEntry {
            href: href.to_string(),
            name,
            is_dir,
            size: captures.get(4).and_then(|m| parse_size(m.as_str())),
            mtime: captures.get(3).map(|m| m.as_str().to_string()),
        });
    }
    entries
}

/// Extrae rutas de directorio del `listado.html` (arbol completo publicado por el sitio).
pub fn parse_listado(html: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for captures in listado_re().captures_iter(html) {
        let path = format!(
            "/{}",
            captures.get(1).map(|m| m.as_str()).unwrap_or_default()
        );
        if !path.ends_with('/') {
            continue;
        }
        if seen.insert(path.clone()) {
            paths.push(path);
        }
    }
    paths
}

pub async fn fetch_text(http: &reqwest::Client, url: &str) -> Result<String, String> {
    let mut last_error = String::from("sin intentos");
    for attempt in 0..3 {
        match http.get(url).send().await {
            Ok(response) if response.status().is_success() => {
                let bytes = response.bytes().await.map_err(|e| e.to_string())?;
                return Ok(String::from_utf8(bytes.to_vec()).unwrap_or_else(|err| {
                    err.as_bytes().iter().map(|byte| *byte as char).collect()
                }));
            }
            Ok(response) => last_error = format!("HTTP {}", response.status()),
            Err(err) => last_error = err.to_string(),
        }
        tokio::time::sleep(std::time::Duration::from_millis(400 * (attempt + 1))).await;
    }
    Err(last_error)
}

/// Lista un directorio remoto en vivo.
pub async fn fetch_listing(http: &reqwest::Client, path: &str) -> Result<Vec<RawEntry>, String> {
    let mut target = path.to_string();
    if !target.ends_with('/') {
        target.push('/');
    }
    let html = fetch_text(http, &absolute_url(&target)).await?;
    Ok(parse_listing(&html))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sizes() {
        assert_eq!(parse_size("3G"), Some(3 * 1024 * 1024 * 1024));
        assert_eq!(parse_size("131K"), Some(131 * 1024));
        assert_eq!(parse_size("-"), None);
    }

    #[test]
    fn listing_rows() {
        let html = r#"<a href="Peliculas/">Peliculas/</a>      29-Sep-2026 05:30       -
<a href="listado.txt">listado.txt</a>                     02-Oct-2026 03:03     1M
<a href="../">../</a>"#;
        let entries = parse_listing(html);
        assert_eq!(entries.len(), 2);
        assert!(entries[0].is_dir);
        assert_eq!(entries[1].size, Some(1024 * 1024));
    }

    #[test]
    fn listado_paths() {
        let html = r#"<a href="http://visuales.uclv.cu//Peliculas/2024/Wicked%20%282024%29/">Wicked (2024)</a>"#;
        assert_eq!(
            parse_listado(html),
            vec!["/Peliculas/2024/Wicked%20%282024%29/"]
        );
    }
}
