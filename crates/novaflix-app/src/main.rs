// En Windows, sin consola en las compilaciones de producción.
#![cfg_attr(all(windows, not(debug_assertions)), windows_subsystem = "windows")]

mod app;
mod card;
mod cover;
mod format;
mod motion;
mod screens;
mod shot;
mod theme;
mod widgets;

use std::sync::Arc;

use app::{Cli, NovaApp, Screen};
use novaflix_core::{Core, CoreOptions};

fn parse_cli() -> Cli {
    let mut cli = Cli::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--screen" => cli.screen = args.next().and_then(|s| Screen::parse(&s)),
            "--open" => cli.open = args.next(),
            "--query" => cli.query = args.next(),
            "--shot" => cli.shot = args.next().map(Into::into),
            "--tab" => cli.tab = args.next(),
            "--light" => cli.light = true,
            _ => {}
        }
    }
    cli
}

fn data_dir() -> std::path::PathBuf {
    // Misma ubicación que v1 para conservar los datos de usuario (Constitución IX).
    if let Ok(dir) = std::env::var("NOVAFLIX_DATA_DIR") {
        return dir.into();
    }
    directories::BaseDirs::new()
        .map(|d| d.data_dir().join("com.novatic.novaflix"))
        .unwrap_or_else(|| std::path::PathBuf::from("novaflix-data"))
}

fn main() -> eframe::Result {
    let cli = parse_cli();
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(4)
        .enable_all()
        .build()
        .expect("no se pudo iniciar el entorno asíncrono");
    let handle = runtime.handle().clone();
    let _guard = runtime.enter();

    let mut viewport = egui::ViewportBuilder::default()
        .with_title("NovaFlix")
        .with_app_id("com.novatic.novaflix")
        .with_inner_size([1180.0, 780.0])
        .with_min_inner_size([960.0, 640.0]);
    if let Ok(icon) = eframe::icon_data::from_png_bytes(include_bytes!("../assets/icons/icon.png"))
    {
        viewport = viewport.with_icon(std::sync::Arc::new(icon));
    }
    if cli.shot.is_some() {
        // En modo captura la ventana no debe robar el foco ni reaccionar al ratón del usuario.
        viewport = viewport
            .with_mouse_passthrough(true)
            .with_active(false)
            .with_taskbar(false);
    }
    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "NovaFlix",
        options,
        Box::new(move |cc| {
            let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
            let ctx = cc.egui_ctx.clone();
            let core = match Core::open(CoreOptions {
                data_dir: data_dir(),
                events: tx,
                wake: Arc::new(move || ctx.request_repaint()),
                runtime: handle,
                base_url: None,
            }) {
                Ok(core) => core,
                Err(err) => {
                    eprintln!("NovaFlix no pudo abrir sus datos: {err}");
                    std::process::exit(1);
                }
            };
            Ok(Box::new(NovaApp::new(cc, core, rx, cli)))
        }),
    )
}
