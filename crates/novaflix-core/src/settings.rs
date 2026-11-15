//! Ajustes tipados sobre la tabla `settings` (claves en data-model.md §4.7).
//! Las contraseñas NO se guardan aquí (Constitución I).

use std::path::PathBuf;

use rusqlite::Connection;

use crate::{
    db,
    error::{Error, Result},
    transport::{Mode, ProxyChoice},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IndexMode {
    #[default]
    Light,
    Deep,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Theme {
    System,
    #[default]
    Dark,
    Light,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tri {
    #[default]
    System,
    On,
    Off,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub net_mode: Mode,
    pub proxy: ProxyChoice,
    pub proxy_user: String,
    pub ftp_user: String,
    pub max_active_downloads: u8,
    pub speed_limit_kbps: u32,
    pub download_dir: Option<PathBuf>,
    pub player: String,
    pub cache_ttl_hours: u32,
    pub cache_max_mb: u32,
    pub bootstrap_ttl_hours: u32,
    pub index_mode: IndexMode,
    pub theme: Theme,
    pub ui_scale: f32,
    pub reduce_motion: Tri,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            net_mode: Mode::Auto,
            proxy: ProxyChoice::System,
            proxy_user: String::new(),
            ftp_user: "anonymous".into(),
            max_active_downloads: 1,
            speed_limit_kbps: 0,
            download_dir: None,
            player: "auto".into(),
            cache_ttl_hours: 12,
            cache_max_mb: 64,
            bootstrap_ttl_hours: 24,
            index_mode: IndexMode::Light,
            theme: Theme::Dark,
            ui_scale: 1.0,
            reduce_motion: Tri::System,
        }
    }
}

fn parse<T: std::str::FromStr>(conn: &Connection, key: &str, fallback: T) -> T {
    db::get_setting(conn, key)
        .and_then(|v| v.parse().ok())
        .unwrap_or(fallback)
}

impl Settings {
    pub fn load(conn: &Connection) -> Self {
        let d = Settings::default();
        let proxy_url = db::get_setting(conn, "proxy_url").unwrap_or_default();
        Settings {
            net_mode: Mode::parse(&db::get_setting(conn, "net_mode").unwrap_or_default()),
            proxy: match db::get_setting(conn, "proxy_mode").as_deref() {
                Some("direct") => ProxyChoice::Direct,
                Some("custom") if !proxy_url.is_empty() => ProxyChoice::Custom(proxy_url),
                _ => ProxyChoice::System,
            },
            proxy_user: db::get_setting(conn, "proxy_user").unwrap_or_default(),
            ftp_user: db::get_setting(conn, "ftp_user")
                .filter(|u| !u.is_empty())
                .unwrap_or(d.ftp_user),
            max_active_downloads: parse(conn, "max_active_downloads", d.max_active_downloads)
                .clamp(1, 3),
            speed_limit_kbps: parse(conn, "speed_limit_kbps", d.speed_limit_kbps),
            download_dir: db::get_setting(conn, "download_dir")
                .filter(|v| !v.is_empty())
                .map(PathBuf::from),
            player: db::get_setting(conn, "player").unwrap_or(d.player),
            cache_ttl_hours: parse(conn, "cache_ttl_hours", d.cache_ttl_hours),
            cache_max_mb: parse(conn, "cache_max_mb", d.cache_max_mb),
            bootstrap_ttl_hours: parse(conn, "bootstrap_ttl_hours", d.bootstrap_ttl_hours),
            index_mode: match db::get_setting(conn, "index_mode").as_deref() {
                Some("deep") => IndexMode::Deep,
                _ => IndexMode::Light,
            },
            theme: match db::get_setting(conn, "theme").as_deref() {
                Some("system") => Theme::System,
                Some("light") => Theme::Light,
                _ => Theme::Dark,
            },
            ui_scale: parse(conn, "ui_scale", d.ui_scale).clamp(0.8, 2.0),
            reduce_motion: match db::get_setting(conn, "reduce_motion").as_deref() {
                Some("on") => Tri::On,
                Some("off") => Tri::Off,
                _ => Tri::System,
            },
        }
    }

    /// Valida los campos sin guardar nada. El error nombra el campo afectado.
    pub fn validate(&self) -> Result<()> {
        if !(1..=3).contains(&self.max_active_downloads) {
            return Err(Error::msg(
                "Las descargas simultáneas deben ser entre 1 y 3.",
            ));
        }
        if !(0.8..=2.0).contains(&self.ui_scale) {
            return Err(Error::msg(
                "La escala de la interfaz debe estar entre 0,8 y 2,0.",
            ));
        }
        if let ProxyChoice::Custom(url) = &self.proxy {
            let parsed =
                url::Url::parse(url).map_err(|_| Error::msg("El proxy no es una URL válida."))?;
            if !matches!(parsed.scheme(), "http" | "https" | "socks5" | "socks5h") {
                return Err(Error::msg(
                    "El proxy debe empezar por http://, https:// o socks5://.",
                ));
            }
            if parsed.password().is_some() {
                return Err(Error::msg(
                    "No escribas la contraseña en la URL del proxy: se pide aparte y no se guarda.",
                ));
            }
        }
        if self.cache_max_mb == 0 || self.cache_ttl_hours == 0 {
            return Err(Error::msg(
                "La caché necesita un tamaño y una vigencia mayores que cero.",
            ));
        }
        Ok(())
    }

    /// Guarda todo en una transacción. Llama antes a `validate`.
    pub fn save(&self, conn: &Connection) -> Result<()> {
        let (proxy_mode, proxy_url) = match &self.proxy {
            ProxyChoice::System => ("system", String::new()),
            ProxyChoice::Direct => ("direct", String::new()),
            ProxyChoice::Custom(url) => ("custom", url.clone()),
        };
        let theme = match self.theme {
            Theme::System => "system",
            Theme::Dark => "dark",
            Theme::Light => "light",
        };
        let motion = match self.reduce_motion {
            Tri::System => "system",
            Tri::On => "on",
            Tri::Off => "off",
        };
        let pairs: Vec<(&str, String)> = vec![
            ("net_mode", self.net_mode.as_str().into()),
            ("proxy_mode", proxy_mode.into()),
            ("proxy_url", proxy_url),
            ("proxy_user", self.proxy_user.clone()),
            ("ftp_user", self.ftp_user.clone()),
            (
                "max_active_downloads",
                self.max_active_downloads.to_string(),
            ),
            ("speed_limit_kbps", self.speed_limit_kbps.to_string()),
            (
                "download_dir",
                self.download_dir
                    .as_ref()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ),
            ("player", self.player.clone()),
            ("cache_ttl_hours", self.cache_ttl_hours.to_string()),
            ("cache_max_mb", self.cache_max_mb.to_string()),
            ("bootstrap_ttl_hours", self.bootstrap_ttl_hours.to_string()),
            (
                "index_mode",
                match self.index_mode {
                    IndexMode::Light => "light".into(),
                    IndexMode::Deep => "deep".into(),
                },
            ),
            ("theme", theme.into()),
            ("ui_scale", self.ui_scale.to_string()),
            ("reduce_motion", motion.into()),
        ];
        conn.execute_batch("BEGIN")?;
        for (key, value) in &pairs {
            if let Err(err) = db::set_setting(conn, key, value) {
                let _ = conn.execute_batch("ROLLBACK");
                return Err(err);
            }
        }
        conn.execute_batch("COMMIT")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn conn() -> Connection {
        let dir = tempfile::tempdir().unwrap();
        let conn = db::open_app(&dir.path().join("app.db")).unwrap();
        std::mem::forget(dir); // la base vive lo que dure la prueba
        conn
    }

    #[test]
    fn defaults_roundtrip() {
        let conn = conn();
        let s = Settings::default();
        s.save(&conn).unwrap();
        assert_eq!(Settings::load(&conn), s);
    }

    #[test]
    fn custom_values_roundtrip() {
        let conn = conn();
        let s = Settings {
            net_mode: Mode::Http,
            proxy: ProxyChoice::Custom("socks5://10.0.0.1:9050".into()),
            max_active_downloads: 2,
            theme: Theme::Light,
            ui_scale: 1.25,
            ..Settings::default()
        };
        s.validate().unwrap();
        s.save(&conn).unwrap();
        assert_eq!(Settings::load(&conn), s);
    }

    #[test]
    fn rejects_invalid_values() {
        let bad = Settings {
            max_active_downloads: 9,
            ..Settings::default()
        };
        assert!(bad
            .validate()
            .unwrap_err()
            .to_string()
            .contains("entre 1 y 3"));

        let bad = Settings {
            proxy: ProxyChoice::Custom("ftp://x".into()),
            ..Settings::default()
        };
        assert!(bad.validate().unwrap_err().to_string().contains("socks5"));

        let bad = Settings {
            proxy: ProxyChoice::Custom("http://ana:clave@h:3128".into()),
            ..Settings::default()
        };
        assert!(bad
            .validate()
            .unwrap_err()
            .to_string()
            .contains("contraseña"));
    }

    #[test]
    fn passwords_are_never_persisted() {
        let conn = conn();
        Settings::default().save(&conn).unwrap();
        let mut stmt = conn.prepare("SELECT key FROM settings").unwrap();
        let keys: Vec<String> = stmt
            .query_map([], |r| r.get(0))
            .unwrap()
            .flatten()
            .collect();
        assert!(
            keys.iter()
                .all(|k| !k.contains("pass") && !k.contains("secret")),
            "{keys:?}"
        );
    }
}
