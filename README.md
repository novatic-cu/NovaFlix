# NovaFlix

Cliente de escritorio para buscar en el catálogo público de **visuales.uclv.cu**: búsqueda instantánea, favoritos, listas de reproducción, lista de "para ver", gestor de descargas y contador de descargas por contenido.

No aloja contenido. Indexa el listado público del sitio en una caché local desechable y descarga solo cuando tú lo pides.

## Stack

- **Tauri 2** (Rust) + **React 19** + TypeScript + Vite + Tailwind v4
- **SQLite** (`rusqlite`, FTS5) con dos bases separadas: caché y datos personales
- `reqwest` (rustls), `motion` (springs), `lucide-react`

## Requisitos

- Node.js 24 + pnpm 12 (Corepack)
- Rust 1.77+
- Linux: `webkit2gtk-4.1`, `gtk3`, `libssl`, `openssl`, `libayatana-appindicator3`, `librsvg`, `patchelf` (patchelf solo para AppImage)
- Windows: WebView2 (incluido en Windows 10/11) + MSVC Build Tools

## Desarrollo

```bash
pnpm install
pnpm tauri dev
```

## Build

```bash
pnpm tauri build --bundles deb        # Linux .deb
pnpm tauri build --bundles appimage   # Linux AppImage
pnpm tauri build --bundles nsis       # Windows .exe
pnpm tauri build --bundles msi        # Windows .msi
```

Cada sistema compila su propio instalador; no hay compilación cruzada. El workflow `.github/workflows/build.yml` genera ambos en CI.

## Arquitectura

```
src/                     React
  components/            Sidebar, Sheet, ItemRow, Toasts, AddToList
  screens/               Search, Browse, Favorites, Lists, Downloads, Settings
  lib/                   ipc tipado, acciones, formato, tipos
  store.ts               estado de UI (Zustand)
src-tauri/src/
  commands.rs            superficie IPC
  index.rs               bootstrap, búsqueda FTS, browse, crawl
  db.rs                  esquema y helpers
  visuales.rs            cliente HTTP + parser del autoindex
  downloads.rs           cola con Range/resume y eventos
  library.rs             favoritos, listas, contadores, M3U, reproductor
  norm.rs                normalización leet + año
  state.rs               estado compartido
```

### Índice y caché

| Capa | Contenido | Coste |
|---|---|---|
| Bootstrap | `listado.html` → 30k carpetas | 1 request, ~6 MB |
| On-demand | listados reales al abrir carpeta | TTL 12 h, LRU 64 MB |
| Profundo | crawl BFS completo (opt-in) | 50–70 min, ~60–90 MB |

`cache.db` es descartable: se puede vaciar desde Ajustes sin perder nada personal. `app.db` guarda favoritos, listas, contadores, historial y ajustes.

### Búsqueda

FTS5 con prefijos sobre `dir_fts` y `file_fts`. La normalización convierte `Wick3d.For.Good.2025.1080p` en `wicked for good 2025`, de modo que buscar `wicked` encuentra variantes escritas para evadir filtros. Los años no se ven afectados por el mapeo leet.

### Descargas

Cola con concurrencia 1, reanudación vía `Range` sobre `.part`, progreso por evento (`download://progress`) y contadores `attempts` / `completed` por ruta.

### Reproducción

Siempre externa: `vlc`, `mpv`, `celluloid` o `totem` (configurable). El webview no reproduce `.mkv` con x265/AC3. Para listas se exporta M3U/M3U8 con URLs absolutas.

## Atajos

| Atajo | Acción |
|---|---|
| `Ctrl/⌘ + K` | Ir a Buscar y enfocar el campo |
| `Esc` | Cerrar hoja modal |
| Doble clic | Abrir carpeta / reproducir archivo |

## Datos locales

- Linux: `~/.local/share/com.novatic.novaflix/`
- Windows: `%APPDATA%\com.novatic.novaflix\`

## Aviso

NovaFlix no aloja ni redistribuye contenido. Solo indexa un listado HTTP público; la descarga y el uso de los archivos es responsabilidad del usuario.
