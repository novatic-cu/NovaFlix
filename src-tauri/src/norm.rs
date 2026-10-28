const RELEASE_TOKENS: &[&str] = &[
    "1080p", "720p", "2160p", "480p", "4k", "uhd", "x265", "x264", "h264", "h265", "hevc", "av1",
    "webrip", "web", "webdl", "bluray", "blu", "ray", "brrip", "bdrip", "dvdrip", "hdrip", "yts",
    "yify", "ytsmx", "aac", "ac3", "dts", "ddp", "hdr", "10bit", "8bit", "repack", "proper",
    "extended", "remux", "sub", "subs", "srt", "mkv", "mp4", "avi", "nfo", "com", "es", "en",
];

/// Normaliza un nombre para busqueda: minusculas, leet-speak, sin separadores
/// y sin tokens de release. `Wick3d.For.Good.2025.1080p` -> `wicked for good 2025`
pub fn normalize(input: &str) -> String {
    let lower = input.to_lowercase();

    // 1) Normaliza acentos y separadores a espacios.
    let mut cleaned = String::with_capacity(lower.len());
    for ch in lower.chars() {
        let plain = match ch {
            'á' | 'à' | 'ä' | 'â' => 'a',
            'é' | 'è' | 'ë' | 'ê' => 'e',
            'í' | 'ì' | 'ï' | 'î' => 'i',
            'ó' | 'ò' | 'ö' | 'ô' => 'o',
            'ú' | 'ù' | 'ü' | 'û' => 'u',
            'ñ' => 'n',
            'ç' => 'c',
            other => other,
        };
        if plain.is_alphanumeric() {
            cleaned.push(plain);
        } else {
            cleaned.push(' ');
        }
    }

    // 2) Leet-speak solo en tokens con letras: los años y cifras se conservan.
    let mut tokens: Vec<String> = Vec::new();
    for token in cleaned.split_whitespace() {
        if token.len() < 2 || RELEASE_TOKENS.contains(&token) {
            continue;
        }
        if token.chars().all(|ch| ch.is_ascii_digit()) {
            tokens.push(token.to_string());
            continue;
        }
        let mapped: String = token
            .chars()
            .map(|ch| match ch {
                '3' => 'e',
                '0' => 'o',
                '1' => 'i',
                '4' => 'a',
                '5' => 's',
                '7' => 't',
                '8' => 'b',
                '@' => 'a',
                '$' => 's',
                '!' => 'i',
                other => other,
            })
            .collect();
        if mapped.len() >= 2 {
            tokens.push(mapped);
        }
    }
    tokens.join(" ")
}

/// Convierte texto libre en una consulta FTS5 con prefijos.
pub fn fts_query(input: &str) -> Option<String> {
    let normalized = normalize(input);
    if normalized.is_empty() {
        return None;
    }
    let parts: Vec<String> = normalized
        .split_whitespace()
        .map(|token| format!("\"{}\"*", token))
        .collect();
    Some(parts.join(" AND "))
}

/// Extrae un año plausible (1900-2099) del nombre.
pub fn extract_year(name: &str) -> Option<i64> {
    let chars: Vec<char> = name.chars().collect();
    for window in chars.windows(4) {
        if window.iter().all(|c| c.is_ascii_digit()) {
            let value: i64 = window.iter().collect::<String>().parse().ok()?;
            if (1900..=2099).contains(&value) {
                return Some(value);
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn leet_and_release_tokens() {
        assert_eq!(normalize("Wick3d For Good 2025"), "wicked for good 2025");
        assert_eq!(
            normalize("The.Good.Dinosaur.2015.720p.BluRay.x264-[YTS.AG]"),
            "the good dinosaur 2015 ag"
        );
        assert_eq!(
            normalize("Dinosaur.2000.720p.BrRip.x264.YIFY.es.srt"),
            "dinosaur 2000"
        );
    }

    #[test]
    fn fts_prefixes() {
        assert_eq!(
            fts_query("wicked 2025"),
            Some("\"wicked\"* AND \"2025\"*".to_string())
        );
        assert_eq!(fts_query("   "), None);
    }

    #[test]
    fn years() {
        assert_eq!(extract_year("(2015) The Good Dinosaur"), Some(2015));
        assert_eq!(extract_year("no year here"), None);
    }
}
