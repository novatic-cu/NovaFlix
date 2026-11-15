//! Pósters: imágenes que el propio servidor guarda en cada carpeta (`poster.jpg`, miniaturas…).
//! Se piden solo al mismo host del catálogo (Constitución I), se guardan en disco y se reducen
//! a miniaturas; si la carpeta no tiene imagen, la interfaz usa su portada generada.

use std::{
    path::PathBuf,
    sync::Arc,
    time::{Duration, SystemTime},
};

use crate::{catalog, index::Entry, Core, Event, Kind};

/// Imagen ya decodificada y reducida (RGBA 8 bits), lista para crear una textura.
#[derive(Debug, Clone)]
pub struct PosterImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

const MAX_BYTES: usize = 6 * 1024 * 1024;
const NEGATIVE_TTL: Duration = Duration::from_secs(3 * 24 * 3600);
const THUMB: (u32, u32) = (384, 576);

fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

/// Puntúa un archivo como candidato a póster (0 = descartado).
fn score(entry: &Entry) -> u32 {
    if entry.kind != Kind::File {
        return 0;
    }
    let name = entry.name.to_lowercase();
    let Some((stem, ext)) = name.rsplit_once('.') else {
        return 0;
    };
    if !matches!(ext, "jpg" | "jpeg" | "png" | "webp") {
        return 0;
    }
    if entry.size.is_some_and(|s| s as usize > MAX_BYTES) || stem.contains("logo") {
        return 0;
    }
    match stem {
        "poster" => 100,
        "folder" => 90,
        "cover" | "front" => 85,
        _ if stem.contains("poster") => 70,
        "fanart" | "backdrop" => 50,
        "landscape" => 45,
        _ if stem.contains("thumb") => 30,
        _ => 10,
    }
}

/// Elige la mejor imagen de una carpeta; a igualdad de puntos, la más pequeña.
pub fn pick_poster(entries: &[Entry]) -> Option<&Entry> {
    entries
        .iter()
        .filter(|e| score(e) > 0)
        .max_by_key(|e| (score(e), std::cmp::Reverse(e.size.unwrap_or(i64::MAX))))
}

/// Título de la serie/película a partir del nombre de un archivo o carpeta de temporada:
/// `Chicago.Fire.S15E06.480p.x264-mSD[EZTVx.to].mkv` → `Chicago Fire`; `Black.Doves.S02` → `Black Doves`.
pub fn show_title(name: &str) -> String {
    let stem = match name.rsplit_once('.') {
        Some((stem, ext)) if ext.len() <= 4 && !ext.contains(' ') => stem,
        _ => name,
    };
    let spaced = stem.replace(['.', '_'], " ");
    let is_marker = |word: &str| {
        let w = word.to_lowercase();
        let season = w.strip_prefix('s').is_some_and(|r| {
            let digits: String = r.chars().take_while(char::is_ascii_digit).collect();
            !digits.is_empty() && (r.len() == digits.len() || r[digits.len()..].starts_with('e'))
        });
        let year =
            w.len() == 4 && w.chars().all(|c| c.is_ascii_digit()) && w.starts_with(['1', '2']);
        let quality = [
            "1080p", "720p", "480p", "2160p", "x264", "x265", "webrip", "web-dl", "bluray", "hdtv",
        ];
        season || year || quality.contains(&w.as_str()) || w.starts_with('[')
    };
    let words: Vec<&str> = spaced
        .split_whitespace()
        .take_while(|w| !is_marker(w))
        .collect();
    words.join(" ")
}

/// Quita las bandas negras (letterbox/pillarbox) de las capturas de vídeo.
fn crop_bars(image: image::DynamicImage) -> image::DynamicImage {
    let rgb = image.to_rgb8();
    let (w, h) = rgb.dimensions();
    if w < 32 || h < 32 {
        return image;
    }
    let luma = |x: u32, y: u32| rgb.get_pixel(x, y).0.iter().map(|c| *c as u32).sum::<u32>();
    // Una fila o columna es «negra» si la media de r+g+b de sus píxeles es < 48.
    let row_dark = |y: u32| (0..w).map(|x| luma(x, y)).sum::<u32>() / w < 48;
    let col_dark = |x: u32| (0..h).map(|y| luma(x, y)).sum::<u32>() / h < 48;
    let top = (0..h / 2).take_while(|y| row_dark(*y)).count() as u32;
    let bottom = (0..h / 2).take_while(|y| row_dark(h - 1 - *y)).count() as u32;
    let left = (0..w / 2).take_while(|x| col_dark(*x)).count() as u32;
    let right = (0..w / 2).take_while(|x| col_dark(w - 1 - *x)).count() as u32;
    let (cw, ch) = (w - left - right, h - top - bottom);
    // Solo si la banda es apreciable y queda una imagen razonable.
    if (top + bottom + left + right) * 100 < (w + h) * 2 || cw < w / 3 || ch < h / 3 {
        return image;
    }
    image.crop_imm(left, top, cw, ch)
}

fn decode(bytes: &[u8]) -> Option<PosterImage> {
    let image = crop_bars(image::load_from_memory(bytes).ok()?);
    let thumb = image.thumbnail(THUMB.0, THUMB.1).to_rgba8();
    Some(PosterImage {
        width: thumb.width(),
        height: thumb.height(),
        rgba: thumb.into_raw(),
    })
}

impl Core {
    fn poster_dir(&self) -> PathBuf {
        self.data_dir.join("posters")
    }

    /// Pide el póster de una carpeta, o de la serie/película a la que pertenece un archivo suelto
    /// (p. ej. un episodio de `/Recientes/`). Emite exactamente un `Event::Poster`.
    pub fn poster(self: &Arc<Self>, path: &str, is_dir: bool) {
        let core = self.clone();
        let path = path.to_string();
        self.rt.spawn(async move {
            let Ok(_permit) = core.poster_sem.clone().acquire_owned().await else {
                return;
            };
            let image = core.load_poster(&path, is_dir).await;
            core.emit(Event::Poster { path, image });
        });
    }

    async fn load_poster(&self, path: &str, is_dir: bool) -> Option<PosterImage> {
        // 1) Las imágenes de la propia carpeta.
        if is_dir {
            if let Some(image) = self.load_from_folder(path).await {
                return Some(image);
            }
        }
        // 2) Carpetas de la serie o película con ese nombre en el índice (episodios sueltos,
        //    temporadas…): se prueba la coincidencia exacta y después las temporadas, y el primer
        //    póster que aparezca se reutiliza para todos sus episodios.
        for target in self.resolve_by_name(path) {
            if target != path {
                if let Some(image) = self.load_from_folder(&target).await {
                    return Some(image);
                }
            }
        }
        None
    }

    /// Candidatos del índice local cuyo nombre coincide con el título que se deduce del nombre de
    /// `path` (sin `SxxExx`, año ni etiquetas de release): primero exactos, luego por prefijo.
    fn resolve_by_name(&self, path: &str) -> Vec<String> {
        let Some(last) = path.trim_end_matches('/').rsplit('/').next() else {
            return Vec::new();
        };
        let title = crate::norm::normalize(&show_title(&catalog::decode_name(last)));
        if title.len() < 3 {
            return Vec::new();
        }
        let Ok(conn) = crate::error::lock(&self.cache) else {
            return Vec::new();
        };
        let mut found: Vec<String> = Vec::new();
        for (op, value) in [
            ("=", title.clone()),
            ("LIKE", format!("{}%", title.replace(['%', '_'], ""))),
        ] {
            for section in [
                "Series",
                "Peliculas",
                "Infantiles",
                "Reality Shows",
                "Documentales",
            ] {
                let sql = format!(
                    "SELECT path FROM dir_index WHERE section = ?1 AND norm {op} ?2 ORDER BY length(path) LIMIT 3"
                );
                let Ok(mut stmt) = conn.prepare(&sql) else {
                    continue;
                };
                let rows =
                    stmt.query_map(rusqlite::params![section, value], |r| r.get::<_, String>(0));
                for row in rows.into_iter().flatten().flatten() {
                    if !found.contains(&row) {
                        found.push(row);
                    }
                }
            }
            if found.len() >= 4 {
                break;
            }
        }
        found.truncate(4);
        found
    }

    async fn load_from_folder(&self, path: &str) -> Option<PosterImage> {
        let key = fnv1a(path);
        let dir = self.poster_dir();
        let (img_file, none_file) = (
            dir.join(format!("{key:016x}.img")),
            dir.join(format!("{key:016x}.none")),
        );

        if let Ok(bytes) = tokio::fs::read(&img_file).await {
            if let Some(image) = tokio::task::spawn_blocking(move || decode(&bytes))
                .await
                .ok()
                .flatten()
            {
                return Some(image);
            }
            let _ = tokio::fs::remove_file(&img_file).await; // archivo corrupto: se vuelve a pedir
        }
        if let Ok(meta) = tokio::fs::metadata(&none_file).await {
            let fresh = meta
                .modified()
                .ok()
                .and_then(|m| SystemTime::now().duration_since(m).ok())
                .is_some_and(|age| age < NEGATIVE_TTL);
            if fresh {
                return None;
            }
        }

        // Listado de la carpeta (caché si es fresca) para descubrir qué imágenes tiene.
        let listing = match self.browse_cached(path) {
            Some(listing) => listing,
            None => self.browse_live(path).await.ok()?,
        };
        let Some(entry) = pick_poster(&listing.entries) else {
            let _ = tokio::fs::create_dir_all(&dir).await;
            let _ = tokio::fs::write(&none_file, b"").await;
            return None;
        };

        // Siempre por HTTP(S), aunque el modo sea FTP: es una imagen suelta del mismo servidor.
        let url = catalog::absolute_url_with(&self.base_url, &entry.path);
        let response = self
            .net()
            .http()
            .get(&url)
            .timeout(Duration::from_secs(40))
            .send()
            .await
            .ok()?;
        if !response.status().is_success() {
            return None;
        }
        let bytes = response.bytes().await.ok()?;
        if bytes.len() > MAX_BYTES {
            return None;
        }
        let _ = tokio::fs::create_dir_all(&dir).await;
        let _ = tokio::fs::write(&img_file, &bytes).await;
        tokio::task::spawn_blocking(move || decode(&bytes))
            .await
            .ok()
            .flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(name: &str, size: i64) -> Entry {
        Entry {
            name: name.into(),
            path: format!("/x/{name}"),
            kind: Kind::File,
            ext: None,
            size: Some(size),
            mtime: None,
        }
    }

    #[test]
    fn prefers_poster_over_fanart_and_ignores_logos() {
        let entries = vec![
            file("fanart.jpg", 1_500_000),
            file("clearlogo.png", 2_000_000),
            file("landscape.jpg", 500_000),
            file("poster.jpg", 1_200_000),
            file("Wicked.mp4", 2_000_000_000),
        ];
        assert_eq!(pick_poster(&entries).unwrap().name, "poster.jpg");
    }

    #[test]
    fn series_folders_fall_back_to_thumbnails() {
        let entries = vec![
            file("Adolescence.S01E01.[]-thumb.jpg", 90_000),
            file("Adolescence.S01E01.mkv", 9_000_000),
        ];
        assert_eq!(
            pick_poster(&entries).unwrap().name,
            "Adolescence.S01E01.[]-thumb.jpg"
        );
    }

    #[test]
    fn nothing_to_pick_without_images_or_with_huge_ones() {
        assert!(pick_poster(&[file("a.mkv", 1), file("a.nfo", 1)]).is_none());
        assert!(pick_poster(&[file("poster.jpg", 50 * 1024 * 1024)]).is_none());
    }

    #[test]
    fn derives_the_show_title_from_episode_and_season_names() {
        assert_eq!(
            show_title("Chicago.Fire.S15E06.480p.x264-mSD[EZTVx.to].mkv"),
            "Chicago Fire"
        );
        assert_eq!(show_title("Black.Doves.S02"), "Black Doves");
        assert_eq!(show_title("Scrubs 2026 S02E08.mkv"), "Scrubs");
        assert_eq!(
            show_title("Georgie.and.Mandys.First.Marriage.S02E05.mkv"),
            "Georgie and Mandys First Marriage"
        );
        assert_eq!(show_title("Wicked.2024.1080p.WEBRip.x265.mp4"), "Wicked");
        assert_eq!(show_title("South Park"), "South Park");
    }

    #[test]
    fn crops_letterbox_bars_from_video_stills() {
        // 640×360 con 60 px de franja negra arriba y abajo y contenido gris en el medio.
        let mut img = image::RgbImage::new(640, 360);
        for (_, y, px) in img.enumerate_pixels_mut() {
            *px = if (60..300).contains(&y) {
                image::Rgb([120, 130, 140])
            } else {
                image::Rgb([0, 0, 0])
            };
        }
        let mut png = Vec::new();
        image::DynamicImage::ImageRgb8(img)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let poster = decode(&png).unwrap();
        assert_eq!(poster.width, 384, "se reduce al ancho máximo");
        assert_eq!(poster.height, 144, "240/640×384: sin las franjas negras");
        let plain = image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            640,
            360,
            image::Rgb([90, 90, 90]),
        ));
        assert_eq!(crop_bars(plain).height(), 360, "sin franjas no se recorta");
    }

    #[test]
    fn decodes_and_shrinks() {
        let mut png = Vec::new();
        image::DynamicImage::new_rgb8(1000, 1500)
            .write_to(&mut std::io::Cursor::new(&mut png), image::ImageFormat::Png)
            .unwrap();
        let poster = decode(&png).unwrap();
        assert_eq!((poster.width, poster.height), (384, 576));
        assert_eq!(poster.rgba.len(), 384 * 576 * 4);
        assert!(decode(b"no es una imagen").is_none());
    }
}
