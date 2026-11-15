use std::sync::OnceLock;

use regex::Regex;

pub const HOST: &str = "visuales.uclv.cu";
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

/// Formato actual del servidor (Apache con `HTMLTable`): fechas ISO y celdas `<td>`.
fn table_re() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r#"<a href="([^"]+)">([^<]*)</a></td><td align="right">(\d{4}-\d{2}-\d{2} \d{2}:\d{2})\s*</td><td align="right">\s*([^<\s]+)\s*</td>"#,
        )
        .expect("regex de tabla invalido")
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

/// Decodifica el cuerpo de una respuesta: UTF-8 y, si no lo es, Latin-1 (el servidor mezcla ambos).
pub fn decode_body(bytes: &[u8]) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_string(),
        Err(_) => bytes.iter().map(|b| *b as char).collect(),
    }
}

/// Codifica un nombre de fichero como lo hace Apache en sus enlaces (`%20`, `%28`...).
pub fn encode_name(name: &str) -> String {
    use percent_encoding::{utf8_percent_encode, AsciiSet, NON_ALPHANUMERIC};
    const KEEP: &AsciiSet = &NON_ALPHANUMERIC
        .remove(b'-')
        .remove(b'.')
        .remove(b'_')
        .remove(b'~');
    utf8_percent_encode(name, KEEP).to_string()
}

pub fn absolute_url(path: &str) -> String {
    absolute_url_with(BASE, path)
}

pub fn absolute_url_with(base: &str, path: &str) -> String {
    let normalized = if path.starts_with('/') {
        path.to_string()
    } else {
        format!("/{}", path)
    };
    format!(
        "{}{}",
        base.trim_end_matches('/'),
        normalized.replace(' ', "%20")
    )
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

/// Deshace las entidades HTML que Apache y `tree` escapan en enlaces y nombres.
pub fn html_unescape(input: &str) -> String {
    if !input.contains('&') {
        return input.to_string();
    }
    let mut out = String::with_capacity(input.len());
    let mut rest = input;
    while let Some(start) = rest.find('&') {
        out.push_str(&rest[..start]);
        rest = &rest[start..];
        let replaced = rest.find(';').filter(|end| *end <= 8).and_then(|end| {
            let entity = &rest[1..end];
            let ch = match entity {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                _ => entity
                    .strip_prefix('#')
                    .and_then(|n| match n.strip_prefix(['x', 'X']) {
                        Some(hex) => u32::from_str_radix(hex, 16).ok(),
                        None => n.parse().ok(),
                    })
                    .and_then(char::from_u32),
            };
            ch.map(|c| (c, end))
        });
        match replaced {
            Some((ch, end)) => {
                out.push(ch);
                rest = &rest[end + 1..];
            }
            None => {
                out.push('&');
                rest = &rest[1..];
            }
        }
    }
    out.push_str(rest);
    out
}

const MONTHS: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];

/// Lleva cualquier fecha del servidor a `YYYY-MM-DD HH:MM` (ordenable como texto).
/// Acepta el formato antiguo `29-Sep-2026 05:30` y el ISO actual.
pub fn normalize_mtime(raw: &str) -> Option<String> {
    let raw = raw.trim();
    let bytes = raw.as_bytes();
    if raw.len() >= 16 && bytes[4] == b'-' && bytes[7] == b'-' {
        return Some(raw[..16].to_string());
    }
    // DD-Mon-YYYY HH:MM
    let (date, time) = raw.split_once(' ')?;
    let mut parts = date.split('-');
    let (day, mon, year) = (parts.next()?, parts.next()?, parts.next()?);
    let month = MONTHS.iter().position(|m| m.eq_ignore_ascii_case(mon))? + 1;
    Some(format!("{year}-{month:02}-{:0>2} {time}", day))
}

pub fn parse_listing(html: &str) -> Vec<RawEntry> {
    let table = parse_table(html);
    if !table.is_empty() {
        return table;
    }
    parse_pre(html)
}

fn entry_from(href: &str, mtime: Option<&str>, size: Option<&str>) -> Option<RawEntry> {
    let href = html_unescape(href);
    if href == "../" || href.starts_with('?') || href.starts_with('/') || href.starts_with("http") {
        return None;
    }
    let is_dir = href.ends_with('/');
    Some(RawEntry {
        name: decode_name(href.trim_end_matches('/')),
        href,
        is_dir,
        size: size.and_then(parse_size),
        mtime: mtime.and_then(normalize_mtime),
    })
}

fn parse_table(html: &str) -> Vec<RawEntry> {
    table_re()
        .captures_iter(html)
        .filter_map(|c| entry_from(&c[1], Some(&c[3]), Some(&c[4])))
        .collect()
}

fn parse_pre(html: &str) -> Vec<RawEntry> {
    row_re()
        .captures_iter(html)
        .filter_map(|c| {
            entry_from(
                &c[1],
                c.get(3).map(|m| m.as_str()),
                c.get(4).map(|m| m.as_str()),
            )
        })
        .collect()
}

/// Extrae rutas de directorio del `listado.html` (arbol completo publicado por el sitio).
pub fn parse_listado(html: &str) -> Vec<String> {
    let mut paths = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for captures in listado_re().captures_iter(html) {
        let path = format!(
            "/{}",
            html_unescape(captures.get(1).map(|m| m.as_str()).unwrap_or_default())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_roundtrip() {
        let encoded = encode_name("Wicked (2024) ñ.mp4");
        assert_eq!(encoded, "Wicked%20%282024%29%20%C3%B1.mp4");
        assert_eq!(decode_name(&encoded), "Wicked (2024) ñ.mp4");
    }

    const REAL_ROOT: &str = include_str!("../tests/fixtures/apache_table_root.html");
    const REAL_RECIENTES: &str = include_str!("../tests/fixtures/apache_table_recientes.html");

    #[test]
    fn parses_the_real_apache_table_format() {
        // HTML real del servidor (2026-11-14): tabla con fechas ISO.
        let root = parse_listing(REAL_ROOT);
        let names: Vec<_> = root.iter().map(|e| e.name.as_str()).collect();
        assert!(
            names.contains(&"Peliculas") && names.contains(&"Series"),
            "{names:?}"
        );
        assert!(names.contains(&"Festivales y Premios de cine"));
        let docs = root.iter().find(|e| e.name == "Documentales").unwrap();
        assert!(docs.is_dir && docs.href == "Documentales/");
        assert_eq!(docs.mtime.as_deref(), Some("2026-11-10 03:17"));
        let txt = root
            .iter()
            .find(|e| e.name == "REDES_SOCIALES.txt")
            .expect("fichero");
        assert!(!txt.is_dir && txt.size.is_some());
        // «Parent Directory» y los enlaces de ordenación no son entradas.
        assert!(root
            .iter()
            .all(|e| e.href != "/" && !e.href.starts_with('?')));
    }

    #[test]
    fn real_listing_with_files_and_encoded_names() {
        let entries = parse_listing(REAL_RECIENTES);
        assert!(entries.len() >= 4, "{entries:?}");
        let mkv = entries
            .iter()
            .find(|e| e.name.starts_with("Abbott.Elementary"))
            .expect("episodio");
        assert!(!mkv.is_dir);
        assert!(
            mkv.name.contains("[EZTVx.to]"),
            "%5b/%5d se decodifican: {}",
            mkv.name
        );
        assert!(
            mkv.href.contains("%5b"),
            "el href conserva la forma canónica"
        );
        assert_eq!(mkv.size, Some(159 * 1024 * 1024));
        let dir = entries
            .iter()
            .find(|e| e.name == "Black.Doves.S02")
            .unwrap();
        assert!(dir.is_dir && dir.size.is_none());
    }

    /// `NOVAFLIX_REAL_LISTADO=/ruta/listado.html cargo test -p novaflix-core -- --ignored real_listado`
    #[test]
    #[ignore = "necesita NOVAFLIX_REAL_LISTADO con una copia de listado.html"]
    fn real_listado_parses_cleanly() {
        let path = std::env::var("NOVAFLIX_REAL_LISTADO").expect("define NOVAFLIX_REAL_LISTADO");
        let html = decode_body(&std::fs::read(path).unwrap());
        let paths = parse_listado(&html);
        assert!(paths.len() > 20_000, "solo {} rutas", paths.len());
        assert!(paths.iter().all(|p| p.starts_with('/') && p.ends_with('/')));
        assert!(
            paths
                .iter()
                .all(|p| !p.contains("&amp;") && !p.contains("&lt;")),
            "quedan entidades HTML"
        );
        assert!(
            paths.iter().any(|p| p.contains("Python%20&%20C")),
            "el & se conserva"
        );
    }

    #[test]
    fn unescapes_html_entities() {
        assert_eq!(html_unescape("Python%20&amp;%20C"), "Python%20&%20C");
        assert_eq!(
            html_unescape("a &lt;b&gt; &quot;c&quot; &#39;d&#x41;"),
            "a <b> \"c\" 'dA"
        );
        assert_eq!(
            html_unescape("sin entidades & suelto"),
            "sin entidades & suelto"
        );
        assert_eq!(html_unescape("&desconocida; fin"), "&desconocida; fin");
    }

    #[test]
    fn listado_paths_are_unescaped() {
        let html = r#"<a href="http://visuales.uclv.cu//Cursos/Java,%20Python%20&amp;%20C/">x</a>"#;
        assert_eq!(parse_listado(html), vec!["/Cursos/Java,%20Python%20&%20C/"]);
    }

    #[test]
    fn dates_are_normalized_to_iso() {
        assert_eq!(
            normalize_mtime("29-Sep-2026 05:30").as_deref(),
            Some("2026-09-29 05:30")
        );
        assert_eq!(
            normalize_mtime("2026-11-10 03:17 ").as_deref(),
            Some("2026-11-10 03:17")
        );
        assert_eq!(normalize_mtime("ayer"), None);
    }

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
        assert_eq!(entries[0].mtime.as_deref(), Some("2026-09-29 05:30"));
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
