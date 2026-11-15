//! Texto para mostrar: títulos limpios, tamaños, fechas.

/// Quita etiquetas de release y separadores técnicos: `Wicked.2024.1080p.WEBRip.x265-[YTS.MX]` → `Wicked`.
pub fn clean_title(name: &str) -> String {
    const CUT: &[&str] = &[
        "1080p", "720p", "2160p", "480p", "4k", "uhd", "x265", "x264", "h264", "h265", "hevc",
        "webrip", "web-dl", "webdl", "bluray", "brrip", "bdrip", "dvdrip", "hdrip", "remux",
        "repack", "proper", "extended",
    ];
    // Quitar corchetes y llaves con su contenido.
    let mut stripped = String::with_capacity(name.len());
    let mut depth = 0_i32;
    for ch in name.chars() {
        match ch {
            '[' | '{' => depth += 1,
            ']' | '}' => depth = (depth - 1).max(0),
            c if depth == 0 => stripped.push(c),
            _ => {}
        }
    }
    let spaced = if stripped.contains(' ') {
        stripped
    } else {
        stripped.replace(['.', '_'], " ")
    };
    let mut words: Vec<&str> = Vec::new();
    for word in spaced.split_whitespace() {
        let lower = word.to_lowercase();
        if CUT.iter().any(|c| lower.contains(c)) {
            break;
        }
        words.push(word);
    }
    // Un año suelto al final (`Wicked 2024`) se muestra aparte, no en el título.
    if words.len() > 1 {
        if let Some(last) = words.last() {
            let digits = last.trim_matches(|c| c == '(' || c == ')');
            if digits.len() == 4 && digits.chars().all(|c| c.is_ascii_digit()) {
                words.pop();
            }
        }
    }
    // Año entre paréntesis al principio: `(2000) Dinosaur`.
    if let Some(first) = words.first() {
        let digits = first.trim_matches(|c| c == '(' || c == ')');
        if words.len() > 1 && digits.len() == 4 && digits.chars().all(|c| c.is_ascii_digit()) {
            words.remove(0);
        }
    }
    let title = words
        .join(" ")
        .trim_matches(|c: char| c == '-' || c == '.' || c.is_whitespace())
        .to_string();
    if title.is_empty() {
        name.to_string()
    } else {
        title
    }
}

pub fn human_bytes(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut value = bytes as f64;
    let mut unit = 0;
    while value >= 1024.0 && unit < UNITS.len() - 1 {
        value /= 1024.0;
        unit += 1;
    }
    if unit == 0 {
        format!("{bytes} B")
    } else if value >= 100.0 {
        format!("{value:.0} {}", UNITS[unit])
    } else {
        format!("{value:.1} {}", UNITS[unit]).replace('.', ",")
    }
}

pub fn human_speed(bps: u64) -> String {
    format!("{}/s", human_bytes(bps))
}

pub fn human_eta(remaining_bytes: u64, bps: u64) -> String {
    if bps == 0 {
        return "—".into();
    }
    let secs = remaining_bytes / bps;
    match secs {
        0..=59 => format!("{secs} s"),
        60..=3599 => format!("{} min", secs / 60),
        _ => format!("{} h {} min", secs / 3600, (secs % 3600) / 60),
    }
}

/// Número con separador de miles a la española: `29994` → `29.994`.
pub fn thousands(n: i64) -> String {
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, ch) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(ch);
    }
    if n < 0 {
        format!("-{out}")
    } else {
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cleans_release_names() {
        assert_eq!(
            clean_title("Wicked.2024.1080p.WEBRip.x265.10bit.AAC5.1-[YTS.MX]"),
            "Wicked"
        );
        assert_eq!(clean_title("Wicked (2024)"), "Wicked");
        assert_eq!(clean_title("(2000) Dinosaur"), "Dinosaur");
        assert_eq!(clean_title("Wicked For Good 2025"), "Wicked For Good");
        assert_eq!(clean_title("The Good Dinosaur"), "The Good Dinosaur");
        assert_eq!(
            clean_title("2024"),
            "2024",
            "nunca devuelve un título vacío"
        );
        assert_eq!(clean_title("Reality Show [HD]"), "Reality Show");
    }

    #[test]
    fn formats_sizes() {
        assert_eq!(human_bytes(512), "512 B");
        assert_eq!(human_bytes(1536), "1,5 KB");
        assert_eq!(human_bytes(3 * 1024 * 1024 * 1024), "3,0 GB");
        assert_eq!(human_bytes(250 * 1024 * 1024), "250 MB");
    }

    #[test]
    fn formats_eta_and_thousands() {
        assert_eq!(human_eta(1000, 0), "—");
        assert_eq!(human_eta(120_000, 1000), "2 min");
        assert_eq!(thousands(29_994), "29.994");
        assert_eq!(thousands(12), "12");
        assert_eq!(thousands(1_234_567), "1.234.567");
    }
}
