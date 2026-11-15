use std::sync::{Mutex, MutexGuard};

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("{0}")]
    Msg(String),
    #[error("red: {0}")]
    Net(String),
    #[error("ftp: {0}")]
    Ftp(String),
    #[error("base de datos: {0}")]
    Db(#[from] rusqlite::Error),
    #[error("disco: {0}")]
    Io(#[from] std::io::Error),
    #[error("http: {0}")]
    Http(#[from] reqwest::Error),
    #[error("json: {0}")]
    Json(#[from] serde_json::Error),
    #[error("estado interno bloqueado")]
    Poisoned,
}

impl Error {
    pub fn msg(text: impl Into<String>) -> Self {
        Error::Msg(text.into())
    }
}

/// Bloquea un `Mutex` convirtiendo el envenenamiento en un `Error`.
pub fn lock<T>(mutex: &Mutex<T>) -> Result<MutexGuard<'_, T>> {
    mutex.lock().map_err(|_| Error::Poisoned)
}

/// Clasificación pensada para la interfaz: decide qué explicación y qué acción mostrar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub enum ErrorKind {
    Offline,
    ProxyAuth,
    ProxyBlocked,
    FtpLogin,
    NotFound,
    Disk,
    Other,
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        let text = self.to_string().to_lowercase();
        match self {
            Error::Io(_) => ErrorKind::Disk,
            Error::Ftp(_) if text.contains("530") => ErrorKind::FtpLogin,
            Error::Ftp(_) if text.contains("550") => ErrorKind::NotFound,
            _ if text.contains("407") => ErrorKind::ProxyAuth,
            _ if text.contains("404") => ErrorKind::NotFound,
            _ if text.contains("403") && text.contains("connect") => ErrorKind::ProxyBlocked,
            Error::Http(e) if e.is_connect() || e.is_timeout() => ErrorKind::Offline,
            Error::Net(_) | Error::Ftp(_)
                if [
                    "tiempo",
                    "connect",
                    "dns",
                    "refused",
                    "resolve",
                    "no responde",
                ]
                .iter()
                .any(|needle| text.contains(needle)) =>
            {
                ErrorKind::Offline
            }
            _ => ErrorKind::Other,
        }
    }
}

/// Error listo para mostrarse: tipo + texto sin trazas.
#[derive(Debug, Clone, serde::Serialize)]
pub struct UiError {
    pub kind: ErrorKind,
    pub message: String,
}

impl From<&Error> for UiError {
    fn from(err: &Error) -> Self {
        Self {
            kind: err.kind(),
            message: err.to_string(),
        }
    }
}

impl From<Error> for UiError {
    fn from(err: Error) -> Self {
        (&err).into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_errors() {
        assert_eq!(
            Error::Net("HTTP 407 Proxy Authentication Required".into()).kind(),
            ErrorKind::ProxyAuth
        );
        assert_eq!(
            Error::Net("HTTP 404 Not Found".into()).kind(),
            ErrorKind::NotFound
        );
        assert_eq!(
            Error::Ftp("530 login incorrect".into()).kind(),
            ErrorKind::FtpLogin
        );
        assert_eq!(
            Error::Ftp("550 no such file".into()).kind(),
            ErrorKind::NotFound
        );
        assert_eq!(
            Error::Ftp("tiempo agotado conectando a x:21".into()).kind(),
            ErrorKind::Offline
        );
        assert_eq!(Error::Msg("otra cosa".into()).kind(), ErrorKind::Other);
        let io = Error::Io(std::io::Error::other("disco lleno"));
        assert_eq!(io.kind(), ErrorKind::Disk);
    }
}
