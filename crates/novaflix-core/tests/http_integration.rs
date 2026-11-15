//! Transporte HTTP: listados, `Range`, redirecciones y errores de proxy, contra un servidor local.

mod support;

use novaflix_core::{
    transport::{collect, Mode, Net, NetConfig, ProxyChoice},
    ErrorKind,
};
use support::httpd::{self, movie_bytes};

fn net(base: &str) -> Net {
    Net::build(NetConfig {
        mode: Mode::Http,
        proxy: ProxyChoice::Direct,
        base_url: base.to_string(),
        ..NetConfig::default()
    })
    .unwrap()
}

#[tokio::test]
async fn lists_an_apache_index() {
    let server = httpd::spawn().await;
    let entries = net(&server.base).list_dir("/").await.unwrap();
    let names: Vec<_> = entries
        .iter()
        .map(|e| (e.name.as_str(), e.is_dir))
        .collect();
    assert_eq!(
        names,
        [("Peliculas", true), ("Series", true), ("REDES.txt", false)]
    );
    assert_eq!(entries[2].size, Some(637));
}

#[tokio::test]
async fn resumes_with_range() {
    let server = httpd::spawn().await;
    let net = net(&server.base);
    let expected = movie_bytes();

    let full = net.open("/Peliculas/Wicked.mp4", 0).await.unwrap();
    assert_eq!(full.total, Some(expected.len() as u64));
    assert!(!full.resumed);
    assert_eq!(collect(full).await.unwrap(), expected);

    let tail = net.open("/Peliculas/Wicked.mp4", 100_000).await.unwrap();
    assert!(tail.resumed, "debe ser 206");
    assert_eq!(
        tail.total,
        Some(expected.len() as u64),
        "total = longitud restante + offset"
    );
    assert_eq!(collect(tail).await.unwrap(), expected[100_000..]);
}

#[tokio::test]
async fn server_without_range_reports_not_resumed() {
    let server = httpd::spawn().await;
    let download = net(&server.base)
        .open("/norange.bin", 50_000)
        .await
        .unwrap();
    assert!(
        !download.resumed,
        "un 200 con offset > 0 no es una reanudación"
    );
}

#[tokio::test]
async fn redirects_to_other_hosts_are_rejected() {
    // CT-08 / FR-017
    let server = httpd::spawn().await;
    let err = net(&server.base)
        .open("/redir", 0)
        .await
        .err()
        .expect("debe fallar");
    assert!(err.to_string().to_lowercase().contains("redirec"), "{err}");
}

#[tokio::test]
async fn proxy_auth_error_is_classified() {
    let server = httpd::spawn().await;
    let err = net(&server.base).open("/proxyauth", 0).await.err().unwrap();
    assert_eq!(err.kind(), ErrorKind::ProxyAuth, "{err}");
    let err = net(&server.base).open("/no-existe", 0).await.err().unwrap();
    assert_eq!(err.kind(), ErrorKind::NotFound, "{err}");
}
