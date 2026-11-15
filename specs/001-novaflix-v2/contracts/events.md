# Contrato: eventos núcleo → interfaz

**Spec**: [../spec.md](../spec.md) · **API**: [core-api.md](core-api.md)

## 1. Mecanismo

- Canal: `tokio::sync::mpsc::unbounded_channel::<Event>()`. El núcleo conserva el `UnboundedSender`; la interfaz el `UnboundedReceiver`.
- Tras enviar un evento el núcleo llama a `wake()` (en la UI: `egui::Context::request_repaint()`).
- La interfaz vacía el canal con `try_recv()` **al inicio de cada fotograma** y aplica los eventos a su estado. No hay retrollamadas desde hilos de trabajo al estado de la UI.
- Los eventos de progreso (`Download`, `CrawlProgress`) se **agrupan**: como máximo uno cada 180 ms por descarga (como en v1) y siempre uno final.

## 2. Catálogo de eventos

```rust
pub enum Event {
    IndexProgress { phase: IndexPhase, state: Progress, dirs: Option<usize> },
    IndexReady(IndexStats),
    CrawlProgress { dirs: usize, queued: usize, files: usize, errors: usize, max: usize },
    Listing { path: String, result: std::result::Result<Listing, UiError> },
    FolderPlan { root: String, result: std::result::Result<FolderPlan, UiError> },
    Probe(Report),
    Download(DownloadEvent),
    Notice(Notice),
}

pub enum IndexPhase { Bootstrap }
pub enum Progress   { Start, Done }

pub struct DownloadEvent {
    pub id: i64,
    pub state: DownloadState,        // Queued | Active | Paused | Done | Error | Cancelled
    pub done: u64,
    pub total: u64,                  // 0 = desconocido
    pub speed_bps: u64,              // media móvil de ~1 s
    pub proto: Mode,                 // Http | Ftp (el realmente usado)
    pub dest: Option<PathBuf>,       // solo en Done
    pub error: Option<UiError>,
}

pub struct UiError { pub kind: ErrorKind, pub message: String }   // message: apto para usuario

pub struct Notice { pub level: Level /* Info|Warn|Error */, pub message: String }
```

## 3. Garantías de orden y entrega

| # | Garantía |
|---|---|
| E1 | Los eventos de una misma descarga llegan en orden. El último de una descarga es `Done`, `Error`, `Cancelled` o `Paused`. |
| E2 | `IndexProgress{Start}` precede siempre a `IndexProgress{Done}` y a `IndexReady`. |
| E3 | Cada llamada a `browse` produce **exactamente un** `Event::Listing` para su `path`. |
| E4 | Cada `run_probe` produce **exactamente un** `Event::Probe`. |
| E5 | Cada `plan_folder_download` produce **exactamente un** `Event::FolderPlan`. |
| E6 | Ningún evento contiene credenciales (G5). |
| E7 | Si el canal se cierra (la UI terminó), los envíos se descartan sin pánico. |

## 4. Tratamiento en la interfaz

| Evento | Efecto en pantalla |
|---|---|
| `IndexProgress{Start}` | Muestra «Preparando el catálogo…» con progreso indeterminado en Inicio |
| `IndexReady` | Rellena estanterías y estado del índice en la barra |
| `Listing` (Ok) | Pinta la carpeta si coincide con la ruta que se está viendo; si no, descarta |
| `Listing` (Err) | Estado de error con texto y acción según `kind` (core-api.md §10) |
| `Probe` | Actualiza Ajustes → Red y el indicador de la barra de estado |
| `Download` | Actualiza anillo, velocidad y estado en Descargas y en la estantería «Continuar» |
| `Notice` | Toast no modal de 4 s (los de nivel Error permanecen hasta cerrarse) |
