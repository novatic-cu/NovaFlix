//! Prueba de humo contra el sitio real: índice, listado y descarga de un fichero pequeño.
//! `cargo run -p novaflix-core --example smoke` (necesita red o proxy; no se ejecuta en CI).

use std::{sync::Arc, time::Duration};

use novaflix_core::{Core, CoreOptions, DownloadState, Event, SearchFilter};

#[tokio::main]
async fn main() {
    let dir = tempfile::tempdir().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let core = Core::open(CoreOptions {
        data_dir: dir.path().join("data"),
        events: tx,
        wake: Arc::new(|| {}),
        runtime: tokio::runtime::Handle::current(),
        base_url: None,
    })
    .unwrap();

    core.bootstrap(true);
    let stats = loop {
        if let Event::IndexReady(s) = rx.recv().await.unwrap() {
            break s;
        }
    };
    println!("índice: {} carpetas", stats.dirs);
    let page = core
        .search("wick3d", &SearchFilter::default(), 5, 0)
        .unwrap();
    println!(
        "búsqueda «wick3d»: {} resultados en {} ms",
        page.total, page.took_ms
    );
    for hit in page.hits.iter().take(3) {
        println!("  · {} ({})", hit.name, hit.path);
    }

    core.browse("/", true);
    loop {
        if let Event::Listing { result, .. } = rx.recv().await.unwrap() {
            let listing = result.expect("listado");
            println!("raíz: {} entradas", listing.entries.len());
            break;
        }
    }

    let out = dir.path().join("out");
    let ids = core
        .download_enqueue(vec!["/REDES_SOCIALES.txt".into()], Some(out.clone()))
        .unwrap();
    let done = tokio::time::timeout(Duration::from_secs(120), async {
        loop {
            if let Event::Download(d) = rx.recv().await.unwrap() {
                if d.id == ids[0] && d.state.is_finished() {
                    return d;
                }
            }
        }
    })
    .await
    .expect("tiempo agotado");
    assert_eq!(done.state, DownloadState::Done, "{:?}", done.error);
    let bytes = std::fs::metadata(out.join("REDES_SOCIALES.txt"))
        .unwrap()
        .len();
    println!("descarga: {bytes} bytes por {:?}", done.proto);
}
