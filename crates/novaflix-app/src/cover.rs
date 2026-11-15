//! Portadas generadas (design-system §5): deterministas, sin red, instantáneas.

use std::collections::HashMap;

use egui::{Color32, ColorImage, Context, TextureHandle, TextureOptions};

pub const W: usize = 192;
pub const H: usize = 288;

/// 12 paletas curadas de tres tonos. Con la capa oscura inferior, el texto blanco cumple AA.
const PALETTES: [[u32; 3]; 12] = [
    [0x12206B, 0x2F6BFF, 0x00B7F0],
    [0x2B1458, 0x7B4DFF, 0xC15BFF],
    [0x3A0F3F, 0xC2257C, 0xFF7A59],
    [0x06323A, 0x00907F, 0x7AE582],
    [0x1D1B4E, 0x4457D8, 0xA0A8FF],
    [0x3B1D0A, 0xB5541C, 0xFFB454],
    [0x0B2447, 0x1A6FB5, 0x5FD0E8],
    [0x2A0A28, 0x8E2DE2, 0x4A00E0],
    [0x14281D, 0x2E8B57, 0xCFE86B],
    [0x3A0D16, 0xB3243F, 0xFF8E72],
    [0x101B3D, 0x3A4FA8, 0xE45CC8],
    [0x0D2B2B, 0x2A7F8F, 0xF2C94C],
];

pub fn fnv1a(text: &str) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in text.as_bytes() {
        hash ^= *byte as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn unpack(hex: u32) -> [f32; 3] {
    [
        (hex >> 16 & 255) as f32,
        (hex >> 8 & 255) as f32,
        (hex & 255) as f32,
    ]
}

fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

/// Genera la imagen de una portada a partir de una semilla (misma semilla → mismos píxeles).
pub fn render(seed: u64) -> ColorImage {
    let palette = PALETTES[(seed % PALETTES.len() as u64) as usize];
    let (c0, c1, c2) = (unpack(palette[0]), unpack(palette[1]), unpack(palette[2]));
    let angle = (45.0 + ((seed >> 8) % 90) as f32).to_radians();
    let (dx, dy) = (angle.cos(), angle.sin());

    // Forma suave: un círculo grande desplazado, y un arco menor.
    let cx = 0.2 + ((seed >> 16) % 60) as f32 / 100.0;
    let cy = 0.15 + ((seed >> 24) % 40) as f32 / 100.0;
    let radius = 0.35 + ((seed >> 32) % 25) as f32 / 100.0;

    let mut pixels = Vec::with_capacity(W * H);
    for y in 0..H {
        for x in 0..W {
            let (u, v) = (x as f32 / (W - 1) as f32, y as f32 / (H - 1) as f32);
            // Degradado de tres paradas a lo largo del ángulo.
            let t = ((u - 0.5) * dx + (v - 0.5) * dy + 0.5).clamp(0.0, 1.0);
            let mut rgb = if t < 0.5 {
                mix(c0, c1, t * 2.0)
            } else {
                mix(c1, c2, (t - 0.5) * 2.0)
            };

            // Círculo luminoso con borde suave.
            let aspect = H as f32 / W as f32;
            let d = (((u - cx) * 1.0).powi(2) + ((v - cy) / aspect).powi(2)).sqrt();
            let glow = (1.0 - ((d - radius).abs() / 0.08)).clamp(0.0, 1.0) * 0.10
                + (1.0 - (d / radius)).clamp(0.0, 1.0) * 0.10;
            rgb = mix(rgb, [255.0, 255.0, 255.0], glow);

            // Capa oscura inferior: garantiza el contraste del texto (hasta 70 %).
            let shade = ((v - 0.40) / 0.60).clamp(0.0, 1.0).powf(1.2) * 0.70;
            rgb = mix(rgb, [4.0, 6.0, 14.0], shade);

            pixels.push(Color32::from_rgb(rgb[0] as u8, rgb[1] as u8, rgb[2] as u8));
        }
    }
    ColorImage {
        size: [W, H],
        pixels,
        source_size: egui::vec2(W as f32, H as f32),
    }
}

/// Tipo de contenido según la sección raíz (para el icono).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Movie,
    Series,
    Music,
    Book,
    Documentary,
    Folder,
}

impl Kind {
    pub fn of_section(section: &str) -> Kind {
        match section {
            "Peliculas" | "Infantiles" => Kind::Movie,
            "Series" | "Reality Shows" => Kind::Series,
            "MP3s" => Kind::Music,
            "Libros" => Kind::Book,
            "Documentales" => Kind::Documentary,
            _ => Kind::Folder,
        }
    }

    pub fn icon(self) -> &'static str {
        use egui_phosphor::regular as i;
        match self {
            Kind::Movie => i::FILM_SLATE,
            Kind::Series => i::TELEVISION,
            Kind::Music => i::MUSIC_NOTES,
            Kind::Book => i::BOOK_OPEN,
            Kind::Documentary => i::FILM_REEL,
            Kind::Folder => i::FOLDER,
        }
    }

    pub fn label(self) -> &'static str {
        match self {
            Kind::Movie => "Película",
            Kind::Series => "Serie",
            Kind::Music => "Música",
            Kind::Book => "Libro",
            Kind::Documentary => "Documental",
            Kind::Folder => "Carpeta",
        }
    }
}

/// Caché de texturas por semilla.
#[derive(Default)]
pub struct Covers {
    textures: HashMap<u64, TextureHandle>,
}

impl Covers {
    pub fn texture(&mut self, ctx: &Context, name: &str) -> &TextureHandle {
        let seed = fnv1a(&novaflix_core::norm::normalize(name));
        self.textures.entry(seed).or_insert_with(|| {
            ctx.load_texture(
                format!("cover-{seed:x}"),
                render(seed),
                TextureOptions::LINEAR,
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_name_same_cover() {
        let a = render(fnv1a("wicked"));
        let b = render(fnv1a("wicked"));
        assert_eq!(a.pixels, b.pixels);
        assert_ne!(
            render(fnv1a("wicked")).pixels,
            render(fnv1a("dinosaur")).pixels
        );
    }

    #[test]
    fn covers_are_dark_enough_for_white_text_at_the_bottom() {
        // Cada paleta, en la franja inferior (donde va el texto), debe dar ≥ 4,5 : 1 con blanco.
        // Se prueban varios ángulos de degradado por paleta (el ángulo sale de la semilla).
        for palette in 0..PALETTES.len() as u64 {
            for angle in [0_u64, 22, 45, 67, 89] {
                let seed = palette + (angle << 8);
                let image = render(seed);
                for x in (0..W).step_by(8) {
                    let px = image.pixels[(H - 30) * W + x];
                    let ratio = crate::theme::contrast(Color32::WHITE, px);
                    assert!(
                        ratio >= 4.5,
                        "paleta {palette}, ángulo {angle}, x={x}: {ratio:.2}"
                    );
                }
            }
        }
    }

    #[test]
    fn sections_map_to_kinds() {
        assert_eq!(Kind::of_section("Peliculas"), Kind::Movie);
        assert_eq!(Kind::of_section("MP3s"), Kind::Music);
        assert_eq!(Kind::of_section("LaMochila"), Kind::Folder);
    }
}
