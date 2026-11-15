# Contrato: API pública de `novaflix-core`

**Versión del contrato**: 1 · **Spec**: [../spec.md](../spec.md) · **Eventos**: [events.md](events.md) · **Transporte**: [transport.md](transport.md)

Este contrato es la **única** superficie que la interfaz (y una futura CLI) puede usar. Cambiarlo exige actualizar este documento y las pruebas de contrato.

## 1. Reglas generales

| # | Regla |
|---|---|
| G1 | Los métodos **síncronos** tocan solo SQLite local y retornan en < 50 ms en el caso normal. Pueden llamarse desde el hilo de UI. |
| G2 | Los métodos **asíncronos** (red) se lanzan en el *runtime* del núcleo y devuelven su resultado como un [`Event`](events.md). No bloquean al llamador. |
| G3 | Todo `Result` usa `novaflix_core::Error`; su `Display` es apto para mostrarse al usuario (en español, sin trazas). |
| G4 | Las rutas del catálogo son **canónicas** (plan.md §5.3). Ninguna función acepta rutas del sistema de archivos salvo `dest_dir`. |
| G5 | Ninguna función devuelve ni registra contraseñas (Principio I). |
| G6 | `Core` es `Send + Sync` y se comparte como `Arc<Core>`. |
| G7 | Tras cada evento emitido se invoca la función de despertar (`wake`) para que la UI repinte. |

## 2. Construcción

```rust
pub struct Core { /* … */ }

impl Core {
    /// Abre (y migra si hace falta) las bases, construye la red y deja el núcleo listo.
    pub fn open(opts: CoreOptions) -> Result<Arc<Core>>;
}

pub struct CoreOptions {
    pub data_dir: PathBuf,                       // dónde viven cache.db / app.db
    pub events: UnboundedSender<Event>,          // canal hacia la UI
    pub wake: Arc<dyn Fn() + Send + Sync>,       // pide un repintado
    pub runtime: tokio::runtime::Handle,         // dónde se ejecuta la red
    pub base_url: Option<String>,                // None = sitio oficial; solo para pruebas
}
```

**Postcondiciones de `open`**: esquema en `user_version = 1`; `app.db.v1.bak` creada si había datos de v1; descargas `active/queued` pasadas a `paused`; `Net` construido con los ajustes guardados.

## 3. Información

```rust
pub fn version() -> &'static str;                       // CARGO_PKG_VERSION
impl Core {
    pub fn data_dir(&self) -> &Path;
    pub fn downloads_dir(&self) -> PathBuf;             // ajuste download_dir o <datos>/Downloads
    pub fn base_url(&self) -> &'static str;             // https://visuales.uclv.cu
    pub fn migration_notice(&self) -> Option<String>;   // aviso si la migración falló
}
```

## 4. Ajustes

```rust
pub struct Settings {
    pub net_mode: Mode,                 // Auto | Http | Ftp
    pub proxy: ProxyChoice,             // System | Direct | Custom(url)
    pub proxy_user: String,
    pub ftp_user: String,
    pub max_active_downloads: u8,       // 1..=3
    pub speed_limit_kbps: u32,          // 0 = sin límite
    pub download_dir: Option<PathBuf>,
    pub player: String,                 // "auto" | comando
    pub cache_ttl_hours: u32,
    pub cache_max_mb: u32,
    pub bootstrap_ttl_hours: u32,
    pub index_mode: IndexMode,          // Light | Deep
    pub theme: Theme,                   // System | Dark | Light
    pub ui_scale: f32,                  // 0.8..=2.0
    pub reduce_motion: Tri,             // System | On | Off
}

impl Core {
    pub fn settings(&self) -> Result<Settings>;
    /// Valida, guarda y, si cambió algo de red, reconstruye `Net` y relanza el diagnóstico.
    pub fn save_settings(self: &Arc<Self>, new: &Settings) -> Result<()>;
    /// Contraseña del proxy **solo en memoria** (no se persiste nunca). Reconstruye la red y
    /// relanza el diagnóstico. La integración con el llavero del sistema es la tarea T-044.
    pub fn set_proxy_password(self: &Arc<Self>, password: Option<String>) -> Result<()>;
}
```

**Validaciones de `save_settings`**: `max_active_downloads ∈ 1..=3`; `ui_scale ∈ 0.8..=2.0`; `Custom(url)` debe parsear como URL con esquema `http`, `https` o `socks5`; `download_dir`, si se da, debe existir o poder crearse. Un valor inválido devuelve `Error::Msg` con el campo afectado y **no** guarda nada.

## 5. Red y diagnóstico

```rust
impl Core {
    pub fn net(&self) -> Arc<Net>;                      // instantánea inmutable
    pub fn active_mode(&self) -> Mode;                  // HTTP o FTP efectivo ahora
    pub fn proxy_label(&self) -> Option<String>;        // proxy con contraseña enmascarada
    /// Lanza el diagnóstico; emite Event::Probe(Report). Actualiza `ftp_available` en Net.
    pub fn run_probe(self: &Arc<Self>);
}
```

**Garantía**: `Report.proxy` y todos los `detail` están libres de credenciales.

## 6. Índice y búsqueda

```rust
pub struct SearchFilter { pub section: Option<String>, pub year: Option<i64>,
                          pub kind: Option<Kind>, pub ext: Option<String> }  // Kind = Dir | File

impl Core {
    pub fn index_stats(&self) -> Result<IndexStats>;
    /// Carga `listado.html` en el índice. Emite IndexProgress{start,done} e IndexReady.
    /// Si `force` es falso y el índice está fresco (TTL), solo emite IndexReady.
    pub fn bootstrap(self: &Arc<Self>, force: bool);
    /// Búsqueda FTS5 con prefijos. Síncrona. Registra la consulta en `searches`.
    pub fn search(&self, query: &str, filter: &SearchFilter, limit: i64, offset: i64) -> Result<SearchPage>;
    pub fn recent_searches(&self) -> Result<Vec<String>>;
    pub fn sections(&self) -> Result<Vec<SectionInfo>>;     // raíces con conteo de carpetas
    pub fn recent_items(&self, limit: i64) -> Result<Vec<Hit>>; // por seen_at desc (estantería «Recientes»)
    pub fn cache_clear(&self, keep_index: bool) -> Result<IndexStats>;
    pub fn crawl_start(self: &Arc<Self>, max_dirs: usize);  // emite CrawlProgress
    pub fn crawl_stop(&self);
}
```

**Invariantes de `search`**: devuelve ≤ `limit` resultados; carpetas puntúan después de archivos con `score = 10_000 + rank`; una consulta que normaliza a vacío devuelve página vacía (sin error).

## 7. Exploración

```rust
impl Core {
    /// Lista una carpeta (usa caché si es fresca y !force). Emite Event::Listing{path, result}.
    pub fn browse(self: &Arc<Self>, path: &str, force: bool);
    /// Variante síncrona solo-caché: devuelve el listado si está en caché y fresco.
    pub fn browse_cached(&self, path: &str) -> Option<Listing>;
}
```

`path` se normaliza: vacío → `/`; se añade `/` inicial y final. Efectos secundarios (en una transacción): upsert en `dir_index`/`file_index` y `dir_cache`, expulsión de caché si excede `cache_max_mb`.

## 8. Descargas

```rust
impl Core {
    /// Encola archivos. Devuelve los ids creados. `dest_dir=None` usa el ajuste.
    pub fn download_enqueue(self: &Arc<Self>, paths: Vec<String>, dest_dir: Option<PathBuf>) -> Result<Vec<i64>>;
    /// Calcula el contenido de una carpeta (recursivo) para confirmar antes de encolar.
    pub fn plan_folder_download(self: &Arc<Self>, path: &str);   // emite Event::FolderPlan
    pub fn download_folder(self: &Arc<Self>, plan: &FolderPlan, dest_dir: Option<PathBuf>, on_exists: OnExists) -> Result<Vec<i64>>;
    pub fn download_enqueue_with(self: &Arc<Self>, paths: Vec<String>, dest_dir: Option<PathBuf>, on_exists: OnExists) -> Result<Vec<i64>>;
    pub fn downloads(&self) -> Result<Vec<DownloadRow>>;         // últimas 300
    pub fn download_pause(&self, id: i64);
    pub fn download_resume(self: &Arc<Self>, id: i64) -> Result<()>;
    pub fn download_cancel(&self, id: i64);
    pub fn downloads_clear_finished(&self) -> Result<()>;
}
pub struct FolderPlan { pub root: String, pub files: Vec<PlannedFile>, pub total_bytes: u64 }
pub struct PlannedFile { pub path: String, pub relative: String, pub size: Option<u64> }
```

**Contrato de reanudación**: para un destino `D`, los datos parciales se escriben en `D.part`. Al reanudar, el desplazamiento es `len(D.part)`; si el servidor no lo acepta (`resumed=false`), se trunca `D.part` y se empieza de cero con un `Event::Notice`. Al terminar, si el servidor informó el total y `len(D.part) != total`, la descarga queda en `error` («tamaño distinto»). El renombrado `D.part → D` es lo último que ocurre.

**Colisión de nombres**: `OnExists::{Replace, Skip, Keep}`. `download_enqueue` usa `Keep` (conserva ambos: `nombre (1).ext`); `Skip` no encola y no devuelve id; `Replace` sobrescribe al terminar.

**Concurrencia**: un `Semaphore` de `max_active_downloads` permisos, fijado al abrir el núcleo (un cambio en Ajustes se aplica al reiniciar); el resto queda en `queued`.

**Servidor que ignora la reanudación**: si `Download.resumed` es falso con `offset > 0`, o el flujo FTP delata que ignoró `REST`, el `.part` se descarta y se empieza de cero (con un `Notice` de aviso en el primer caso).

## 9. Biblioteca

```rust
impl Core {
    pub fn favorites(&self) -> Result<Vec<Favorite>>;
    pub fn favorite_add(&self, kind: TargetKind, path: &str, label: &str, note: Option<String>, size: Option<i64>) -> Result<()>;
    pub fn favorite_remove(&self, kind: TargetKind, path: &str) -> Result<()>;
    pub fn is_favorite(&self, kind: TargetKind, path: &str) -> bool;

    pub fn lists(&self) -> Result<Vec<ListSummary>>;
    pub fn list_create(&self, name: &str, kind: ListKind) -> Result<i64>;
    pub fn list_rename(&self, id: i64, name: &str) -> Result<()>;
    pub fn list_delete(&self, id: i64) -> Result<()>;
    pub fn list_items(&self, id: i64) -> Result<Vec<ListItem>>;
    pub fn list_add(&self, id: i64, path: &str, name: Option<String>) -> Result<()>;
    pub fn list_remove(&self, id: i64, path: &str) -> Result<()>;
    pub fn list_reorder(&self, id: i64, paths: Vec<String>) -> Result<()>;
    pub fn list_mark_watched(&self, id: i64, path: &str, watched: bool) -> Result<()>;

    pub fn counters(&self) -> Result<Vec<Counter>>;
    pub fn counter_for(&self, path: &str) -> Result<Counter>;

    pub fn export_m3u(&self, list_id: i64) -> Result<PathBuf>;   // en <datos>/exports/
    pub fn play(&self, path: &str) -> Result<()>;                // Err(Error::Msg("no-player")) si no hay
    pub fn stream_url(&self, path: &str) -> String;              // URL HTTP absoluta (para copiar)
}
```

**Nota sobre `play` y FTP**: los reproductores externos reciben siempre la URL **HTTP(S)** absoluta, incluso en modo FTP (los reproductores gestionan mal las credenciales FTP y el proxy). Esto se documenta en la UI de la ficha.

## 10. Errores

```rust
pub enum Error { Msg(String), Net(String), Ftp(String), Db(..), Io(..), Http(..), Json(..), Poisoned }
```

Clasificación para la UI (`Error::kind()`):

| `ErrorKind` | Cuándo | Acción sugerida en la UI |
|---|---|---|
| `Offline` | DNS/conexión fallida | Abrir diagnóstico |
| `ProxyAuth` | 407 | Ir a Ajustes → Red |
| `ProxyBlocked` | 403 al `CONNECT` | Explicar y usar HTTP |
| `FtpLogin` | 530 | Pedir credenciales |
| `NotFound` | 404 / 550 | Refrescar carpeta |
| `Disk` | E/S local | Mostrar ruta y espacio |
| `Other` | resto | Mostrar texto |

## 11. Pruebas de contrato (obligatorias)

| ID | Verifica |
|---|---|
| CT-01 | `search("wick3d")` devuelve «Wicked» tras indexar el fixture |
| CT-02 | `browse` dos veces: la segunda es `cached = true`; con `force`, `false` |
| CT-03 | `download_enqueue` + corte + `download_resume` termina con bytes idénticos |
| CT-04 | `save_settings` rechaza `max_active_downloads = 9` y no modifica nada |
| CT-05 | `Report` serializado no contiene la contraseña del proxy |
| CT-06 | `open` sobre un `app.db` de v1 migra sin pérdida (SC-005) |
| CT-07 | Nombre remoto `../../etc/passwd` produce un destino dentro de `dest_dir` |
| CT-08 | Redirección a otro host es rechazada |
