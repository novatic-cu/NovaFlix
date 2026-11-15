//! Pruebas de contrato de `Core` (specs/001-novaflix-v2/contracts/core-api.md §11).

mod support;

use std::{path::PathBuf, sync::Arc, time::Duration};

use novaflix_core::{
    Core, CoreOptions, DownloadState, Event, Kind, Listing, ProxyChoice, SearchFilter, Settings,
    UiError,
};
use support::httpd::{self, movie_bytes};
use tokio::{sync::mpsc::UnboundedReceiver, time::timeout};

struct Harness {
    core: Arc<Core>,
    rx: UnboundedReceiver<Event>,
    dir: tempfile::TempDir,
    server: httpd::Httpd,
}

async fn harness() -> Harness {
    let server = httpd::spawn().await;
    let dir = tempfile::tempdir().unwrap();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    let core = Core::open(CoreOptions {
        data_dir: dir.path().to_path_buf(),
        events: tx,
        wake: Arc::new(|| {}),
        runtime: tokio::runtime::Handle::current(),
        base_url: Some(server.base.clone()),
    })
    .unwrap();
    // Sin proxy y solo HTTP: las pruebas no deben depender del entorno de quien las ejecuta.
    let settings = Settings {
        proxy: ProxyChoice::Direct,
        net_mode: novaflix_core::Mode::Http,
        ..Settings::default()
    };
    core.save_settings(&settings).unwrap();
    Harness {
        core,
        rx,
        dir,
        server,
    }
}

impl Harness {
    async fn next_matching<T>(&mut self, mut pick: impl FnMut(Event) -> Option<T>) -> T {
        timeout(Duration::from_secs(20), async {
            loop {
                let event = self.rx.recv().await.expect("canal cerrado");
                if let Some(found) = pick(event) {
                    return found;
                }
            }
        })
        .await
        .expect("tiempo agotado esperando un evento")
    }

    async fn listing(&mut self, path: &str, force: bool) -> Result<Listing, UiError> {
        self.core.browse(path, force);
        let want = path.to_string();
        self.next_matching(move |e| match e {
            Event::Listing { path, result } if path.trim_matches('/') == want.trim_matches('/') => {
                Some(result)
            }
            _ => None,
        })
        .await
    }

    async fn wait_download(&mut self, id: i64) -> novaflix_core::DownloadEvent {
        self.next_matching(move |e| match e {
            Event::Download(d) if d.id == id && d.state.is_finished_or_paused() => Some(d),
            _ => None,
        })
        .await
    }
}

trait StateExt {
    fn is_finished_or_paused(&self) -> bool;
}
impl StateExt for DownloadState {
    fn is_finished_or_paused(&self) -> bool {
        self.is_finished() || *self == DownloadState::Paused
    }
}

fn dest_of(h: &Harness, id: i64) -> PathBuf {
    let row = h
        .core
        .downloads()
        .unwrap()
        .into_iter()
        .find(|r| r.id == id)
        .unwrap();
    PathBuf::from(row.dest)
}

#[tokio::test]
async fn ct01_search_finds_leet_names_after_bootstrap() {
    let mut h = harness().await;
    h.core.bootstrap(true);
    let stats = h
        .next_matching(|e| {
            if let Event::IndexReady(s) = e {
                Some(s)
            } else {
                None
            }
        })
        .await;
    assert_eq!(stats.dirs, 3);

    let page = h
        .core
        .search("wick3d", &SearchFilter::default(), 20, 0)
        .unwrap();
    assert_eq!(page.total, 1);
    assert_eq!(page.hits[0].kind, Kind::Dir);
    assert_eq!(page.hits[0].name, "Wicked (2024)");
    assert_eq!(page.hits[0].year, Some(2024));
    assert!(h
        .core
        .search("   ", &SearchFilter::default(), 20, 0)
        .unwrap()
        .hits
        .is_empty());
    assert_eq!(h.core.recent_searches().unwrap()[0], "wick3d");
    assert_eq!(h.core.sections().unwrap().len(), 2); // Peliculas y Series
}

#[tokio::test]
async fn ct02_browse_uses_cache_unless_forced() {
    let mut h = harness().await;
    let first = h.listing("/", false).await.unwrap();
    assert!(!first.cached);
    assert_eq!(first.entries.len(), 3);

    let hits_after_first = h.server.hits.load(std::sync::atomic::Ordering::Relaxed);
    let second = h.listing("/", false).await.unwrap();
    assert!(second.cached, "la segunda lectura sale de la caché");
    assert_eq!(
        h.server.hits.load(std::sync::atomic::Ordering::Relaxed),
        hits_after_first
    );
    assert!(h.core.browse_cached("/").is_some());

    let forced = h.listing("/", true).await.unwrap();
    assert!(!forced.cached);
    assert!(h.server.hits.load(std::sync::atomic::Ordering::Relaxed) > hits_after_first);
}

#[tokio::test]
async fn ct02b_browse_error_is_one_event_with_a_kind() {
    let mut h = harness().await;
    let err = h.listing("/no-existe", false).await.unwrap_err();
    assert_eq!(
        err.kind,
        novaflix_core::ErrorKind::NotFound,
        "{}",
        err.message
    );
}

#[tokio::test]
async fn ct03_download_completes_and_matches_remote() {
    let mut h = harness().await;
    let out = h.dir.path().join("out");
    let ids = h
        .core
        .download_enqueue(vec!["/Peliculas/Wicked.mp4".into()], Some(out.clone()))
        .unwrap();
    let done = h.wait_download(ids[0]).await;
    assert_eq!(done.state, DownloadState::Done, "{:?}", done.error);
    assert_eq!(
        std::fs::read(out.join("Wicked.mp4")).unwrap(),
        movie_bytes()
    );
    assert!(
        !out.join("Wicked.mp4.part").exists(),
        "el .part se renombra al terminar"
    );
    assert_eq!(
        h.core
            .counter_for("/Peliculas/Wicked.mp4")
            .unwrap()
            .completed,
        1
    );
}

#[tokio::test]
async fn ct03b_resumes_from_an_existing_part_file() {
    let mut h = harness().await;
    let out = h.dir.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    // Simula una descarga cortada a mitad: ya hay 120.000 bytes correctos.
    std::fs::write(out.join("Wicked.mp4.part"), &movie_bytes()[..120_000]).unwrap();

    let ids = h
        .core
        .download_enqueue(vec!["/Peliculas/Wicked.mp4".into()], Some(out.clone()))
        .unwrap();
    let done = h.wait_download(ids[0]).await;
    assert_eq!(done.state, DownloadState::Done, "{:?}", done.error);
    assert_eq!(
        std::fs::read(out.join("Wicked.mp4")).unwrap(),
        movie_bytes(),
        "bytes idénticos"
    );
}

#[tokio::test]
async fn ct03c_server_without_range_restarts_cleanly() {
    let mut h = harness().await;
    let out = h.dir.path().join("out");
    std::fs::create_dir_all(&out).unwrap();
    std::fs::write(out.join("norange.bin.part"), vec![9_u8; 50_000]).unwrap(); // basura previa

    let ids = h
        .core
        .download_enqueue(vec!["/norange.bin".into()], Some(out.clone()))
        .unwrap();
    let done = h.wait_download(ids[0]).await;
    assert_eq!(done.state, DownloadState::Done, "{:?}", done.error);
    assert_eq!(
        std::fs::read(out.join("norange.bin")).unwrap(),
        movie_bytes()
    );
}

#[tokio::test]
async fn ct03d_pause_keeps_the_part_and_resume_finishes() {
    let mut h = harness().await;
    let out = h.dir.path().join("out");
    let ids = h
        .core
        .download_enqueue(vec!["/Peliculas/Wicked.mp4".into()], Some(out.clone()))
        .unwrap();
    h.core.download_pause(ids[0]);
    let first = h.wait_download(ids[0]).await;
    // La pausa puede llegar antes de empezar o a mitad; en ambos casos no se pierde nada.
    if first.state == DownloadState::Paused {
        h.core.download_resume(ids[0]).unwrap();
        let done = h.wait_download(ids[0]).await;
        assert_eq!(done.state, DownloadState::Done, "{:?}", done.error);
    } else {
        assert_eq!(first.state, DownloadState::Done);
    }
    assert_eq!(
        std::fs::read(out.join("Wicked.mp4")).unwrap(),
        movie_bytes()
    );
}

#[tokio::test]
async fn ct07_hostile_names_never_escape_the_download_dir() {
    let h = harness().await;
    let out = h.dir.path().join("out");
    let ids = h
        .core
        .download_enqueue(
            vec![
                "/..%2F..%2Fetc%2Fpasswd".into(),
                "/%2e%2e".into(),
                "/a%5Cb%3Ac.mp4".into(),
            ],
            Some(out.clone()),
        )
        .unwrap();
    assert_eq!(ids.len(), 3);
    for id in ids {
        let dest = dest_of(&h, id);
        assert_eq!(dest.parent().unwrap(), out, "{dest:?}");
        let name = dest.file_name().unwrap().to_string_lossy().into_owned();
        assert!(
            !name.contains(['/', '\\', ':']) && !name.starts_with('.'),
            "{name}"
        );
    }
}

#[tokio::test]
async fn ct04_invalid_settings_are_rejected_without_side_effects() {
    let h = harness().await;
    let before = h.core.settings().unwrap();
    let bad = Settings {
        max_active_downloads: 9,
        ..before.clone()
    };
    let err = h.core.save_settings(&bad).unwrap_err().to_string();
    assert!(err.contains("entre 1 y 3"), "{err}");
    assert_eq!(h.core.settings().unwrap(), before);
}

#[tokio::test]
async fn ct05_diagnostic_report_never_contains_the_proxy_password() {
    let mut h = harness().await;
    let settings = Settings {
        proxy: ProxyChoice::Custom("http://127.0.0.1:1".into()),
        proxy_user: "ana".into(),
        ..h.core.settings().unwrap()
    };
    h.core
        .set_proxy_password(Some("secreto-del-proxy".into()))
        .unwrap();
    h.core.save_settings(&settings).unwrap();
    h.core.run_probe();

    let report = h
        .next_matching(|e| {
            if let Event::Probe(r) = e {
                Some(r)
            } else {
                None
            }
        })
        .await;
    let json = serde_json::to_string(&report).unwrap();
    assert!(!json.contains("secreto-del-proxy"), "{json}");
    assert!(
        report
            .proxy
            .as_deref()
            .unwrap_or_default()
            .contains("ana:***@"),
        "{:?}",
        report.proxy
    );
    assert!(!h.core.proxy_label().unwrap().contains("secreto"));
}

#[tokio::test]
async fn ct06_core_open_migrates_v1_data() {
    // La migración en sí se prueba en `migrate::tests`; aquí, que `Core::open` la invoque.
    let dir = tempfile::tempdir().unwrap();
    let conn = rusqlite::Connection::open(dir.path().join("app.db")).unwrap();
    conn.execute_batch(
        "CREATE TABLE items(path TEXT PRIMARY KEY, name TEXT NOT NULL, kind TEXT NOT NULL, ext TEXT,
            section TEXT, year INTEGER, size INTEGER, first_seen TEXT NOT NULL, last_seen TEXT NOT NULL);
         CREATE TABLE favorites(id INTEGER PRIMARY KEY AUTOINCREMENT, target_kind TEXT NOT NULL,
            path TEXT NOT NULL, label TEXT, note TEXT, created_at TEXT NOT NULL, UNIQUE(target_kind, path));
         CREATE TABLE downloads(id INTEGER PRIMARY KEY AUTOINCREMENT, path TEXT NOT NULL, dest TEXT NOT NULL,
            bytes_total INTEGER DEFAULT 0, bytes_done INTEGER DEFAULT 0, state TEXT NOT NULL,
            started_at TEXT NOT NULL, finished_at TEXT, error TEXT);
         INSERT INTO favorites(target_kind,path,label,created_at) VALUES('folder','/Series/X/','X','2026-10-30');
         INSERT INTO downloads(path,dest,state,started_at) VALUES('/a','/tmp/a','active','2026-10-30');",
    )
    .unwrap();
    drop(conn);

    let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
    let core = Core::open(CoreOptions {
        data_dir: dir.path().to_path_buf(),
        events: tx,
        wake: Arc::new(|| {}),
        runtime: tokio::runtime::Handle::current(),
        base_url: None,
    })
    .unwrap();
    assert!(dir.path().join("app.db.v1.bak").exists());
    assert_eq!(core.favorites().unwrap().len(), 1);
    assert_eq!(core.downloads().unwrap()[0].state, DownloadState::Paused);
    assert!(core.migration_notice().is_none());
}
