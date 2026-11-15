//! Pruebas de integración del cliente FTP contra un servidor FTP real (local, efímero).

mod support;

use novaflix_core::transport::{collect, ftp, FtpConfig};
use support::FtpdOptions;

fn fixture() -> tempfile::TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("Peliculas/Wicked (2024)")).unwrap();
    std::fs::write(dir.path().join("REDES_SOCIALES.txt"), b"hola mundo").unwrap();
    let payload: Vec<u8> = (0..200_000u32).map(|i| (i % 251) as u8).collect();
    std::fs::write(
        dir.path().join("Peliculas/Wicked (2024)/Wicked.mp4"),
        payload,
    )
    .unwrap();
    dir
}

fn cfg(port: u16) -> FtpConfig {
    FtpConfig {
        host: "127.0.0.1".into(),
        port,
        ..FtpConfig::default()
    }
}

#[tokio::test]
async fn lists_with_mlsd() {
    let dir = fixture();
    let port = support::spawn(dir.path().to_path_buf(), FtpdOptions::default()).await;

    let root = ftp::list_dir(&cfg(port), "/").await.unwrap();
    let names: Vec<_> = root.iter().map(|e| e.name.as_str()).collect();
    assert_eq!(names, ["Peliculas", "REDES_SOCIALES.txt"]);
    assert!(root[0].is_dir && root[0].href == "Peliculas/");
    assert_eq!(root[1].size, Some(10));

    // Ruta canónica codificada, como la guarda el catálogo.
    let files = ftp::list_dir(&cfg(port), "/Peliculas/Wicked%20%282024%29/")
        .await
        .unwrap();
    assert_eq!(files.len(), 1);
    assert_eq!(files[0].name, "Wicked.mp4");
    assert_eq!(files[0].size, Some(200_000));
}

#[tokio::test]
async fn falls_back_to_list() {
    let dir = fixture();
    let opts = FtpdOptions {
        mlsd: false,
        ..Default::default()
    };
    let port = support::spawn(dir.path().to_path_buf(), opts).await;

    let root = ftp::list_dir(&cfg(port), "/").await.unwrap();
    assert_eq!(root.len(), 2);
    assert!(root.iter().any(|e| e.is_dir && e.name == "Peliculas"));
    assert!(root.iter().any(|e| !e.is_dir && e.size == Some(10)));
}

#[tokio::test]
async fn downloads_and_resumes() {
    let dir = fixture();
    let port = support::spawn(dir.path().to_path_buf(), FtpdOptions::default()).await;
    let expected = std::fs::read(dir.path().join("Peliculas/Wicked (2024)/Wicked.mp4")).unwrap();
    let path = "/Peliculas/Wicked%20%282024%29/Wicked.mp4";

    let full = ftp::open(&cfg(port), path, 0).await.unwrap();
    assert_eq!(full.total, Some(200_000));
    assert!(!full.resumed);
    assert_eq!(collect(full).await.unwrap(), expected);

    let tail = ftp::open(&cfg(port), path, 150_000).await.unwrap();
    assert!(tail.resumed);
    assert_eq!(tail.total, Some(200_000));
    assert_eq!(collect(tail).await.unwrap(), expected[150_000..]);
}

#[tokio::test]
async fn rejects_bad_login_and_missing_files() {
    let dir = fixture();
    let opts = FtpdOptions {
        require_login: Some(("ana".into(), "secreto".into())),
        ..Default::default()
    };
    let port = support::spawn(dir.path().to_path_buf(), opts).await;

    let err = ftp::list_dir(&cfg(port), "/")
        .await
        .unwrap_err()
        .to_string();
    assert!(err.contains("530"), "{err}");

    let good = FtpConfig {
        user: "ana".into(),
        pass: "secreto".into(),
        ..cfg(port)
    };
    assert!(ftp::list_dir(&good, "/").await.is_ok());
    let err = ftp::open(&good, "/no-existe.mp4", 0)
        .await
        .err()
        .unwrap()
        .to_string();
    assert!(err.contains("550"), "{err}");
}

/// Contra un servidor FTP externo e independiente (p. ej. `rclone serve ftp`).
/// `NOVAFLIX_TEST_FTP=127.0.0.1:2121 cargo test -p novaflix-core --test ftp_integration -- --ignored`
#[tokio::test]
#[ignore = "necesita NOVAFLIX_TEST_FTP=host:puerto"]
async fn against_external_server() {
    let addr = std::env::var("NOVAFLIX_TEST_FTP").expect("define NOVAFLIX_TEST_FTP");
    let (host, port) = addr.rsplit_once(':').unwrap();
    let cfg = FtpConfig {
        host: host.into(),
        port: port.parse().unwrap(),
        ..FtpConfig::default()
    };

    let root = ftp::list_dir(&cfg, "/").await.unwrap();
    assert!(
        root.iter().any(|e| e.is_dir && e.name == "Peliculas"),
        "{root:?}"
    );
    let txt = root
        .iter()
        .find(|e| e.name == "REDES_SOCIALES.txt")
        .expect("fichero conocido");

    let body = collect(ftp::open(&cfg, &format!("/{}", txt.href), 0).await.unwrap())
        .await
        .unwrap();
    assert_eq!(Some(body.len() as i64), txt.size);
    // rclone acepta REST pero reenvía desde el inicio: el cliente debe detectarlo, no corromper.
    let resumed = ftp::open(&cfg, &format!("/{}", txt.href), 100)
        .await
        .unwrap();
    match collect(resumed).await {
        Ok(tail) => assert_eq!(tail, body[100..]),
        Err(err) => assert!(err.to_string().contains("REST"), "{err}"),
    }
}
