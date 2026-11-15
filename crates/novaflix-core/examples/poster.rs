//! Pide pósters al sitio real sin interfaz: `cargo run -p novaflix-core --example poster -- /Ruta/Carpeta/ …`

use std::{sync::Arc, time::Instant};

use novaflix_core::{Core, CoreOptions, Event};

#[tokio::main]
async fn main() {
    let paths: Vec<String> = std::env::args().skip(1).collect();
    let dir = tempfile::tempdir().unwrap();
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let core = Core::open(CoreOptions {
        data_dir: std::env::var("NOVAFLIX_DATA_DIR")
            .map(Into::into)
            .unwrap_or_else(|_| dir.path().join("data")),
        events: tx,
        wake: Arc::new(|| {}),
        runtime: tokio::runtime::Handle::current(),
        base_url: None,
    })
    .unwrap();
    for round in ["red", "caché de disco"] {
        for path in &paths {
            let started = Instant::now();
            core.poster(path, path.ends_with('/'));
            loop {
                if let Event::Poster { path: p, image } = rx.recv().await.unwrap() {
                    if &p == path {
                        match image {
                            Some(i) => println!(
                                "[{round}] {path}: {}×{} en {:?}",
                                i.width,
                                i.height,
                                started.elapsed()
                            ),
                            None => {
                                println!("[{round}] {path}: sin póster en {:?}", started.elapsed())
                            }
                        }
                        break;
                    }
                }
            }
        }
    }
    println!(
        "archivos en caché: {}",
        std::fs::read_dir(
            std::env::var("NOVAFLIX_DATA_DIR")
                .map(|d| std::path::PathBuf::from(d).join("posters"))
                .unwrap_or_else(|_| dir.path().join("data/posters"))
        )
        .map(|d| d.count())
        .unwrap_or(0)
    );
}
