# Modelo de datos: NovaFlix v2

**Fecha**: 2026-11-13 · **Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

El esquema de v1 se **conserva** (FR-005, FR-050) y se extiende con cambios aditivos. El esquema base se tomó de `git show v1:src-tauri/src/db.rs`.

## 1. Ubicación y ciclo de vida

| Archivo | Contenido | Tamaño típico | Vida | Copia de seguridad |
|---|---|---|---|---|
| `cache.db` | Índice de rutas, listados visitados | 6–90 MB | Desechable (se reconstruye) | No |
| `app.db` | Favoritos, listas, contadores, ajustes, descargas, búsquedas | < 5 MB | Datos del usuario | Sí (`app.db.v1.bak` en la migración) |

- Linux: `~/.local/share/com.novatic.novaflix/`
- Windows: `%APPDATA%\com.novatic.novaflix\`
- Descargas por defecto: `<datos>/Downloads/` (ajustable).
- Exportaciones M3U: `<datos>/exports/`.

Ambas bases usan `journal_mode = WAL`; `cache.db` además `synchronous = NORMAL`; `app.db` usa `foreign_keys = ON`.

## 2. Versionado del esquema

`PRAGMA user_version` en cada base.

| Valor | Significado |
|---|---|
| `0` | Base creada por v1 (o vacía) |
| `1` | Esquema v2 inicial (este documento) |

Las migraciones son funciones `fn(&Transaction) -> Result<()>` ordenadas; se ejecutan en una única transacción por salto de versión. Una base con `user_version` mayor al soportado **no se abre ni se modifica**: `Core::open` falla con un mensaje claro («los datos son de una versión más nueva; actualiza la aplicación»), lo que evita corromper datos de una versión futura.

## 3. `cache.db`

### 3.1 `meta`
| Columna | Tipo | Notas |
|---|---|---|
| `key` | TEXT PK | `last_bootstrap`, `last_crawl` |
| `value` | TEXT | RFC 3339 |

### 3.2 `dir_index` — carpetas
| Columna | Tipo | Notas |
|---|---|---|
| `path` | TEXT PK | Ruta canónica con `/` final |
| `name` | TEXT | Nombre decodificado |
| `norm` | TEXT | Resultado de `norm::normalize(name)` |
| `parent` | TEXT | Ruta canónica del padre |
| `section` | TEXT | Primer segmento (`Peliculas`, `Series`…) |
| `year` | INTEGER NULL | `norm::extract_year` |
| `source` | TEXT | `bootstrap` \| `browse` \| `crawl` |
| `seen_at` | TEXT | RFC 3339 |

Índice: `idx_dir_section(section, year)`.

### 3.3 `dir_fts` — FTS5 de carpetas
`CREATE VIRTUAL TABLE dir_fts USING fts5(name, norm, path UNINDEXED, tokenize='unicode61 remove_diacritics 2')`. `rowid` = `rowid` de `dir_index`.

### 3.4 `file_index` — archivos
| Columna | Tipo | Notas |
|---|---|---|
| `path` | TEXT PK | Ruta canónica sin `/` final |
| `name`, `norm`, `parent`, `section`, `year` | — | Como en `dir_index` |
| `ext` | TEXT NULL | En minúsculas, ≤ 5 caracteres, sin espacios |
| `size` | INTEGER NULL | Bytes |
| `mtime` | TEXT NULL | Texto tal como lo informa el servidor (`02-Oct-2026 03:03`) |
| `seen_at` | TEXT | RFC 3339 |

Índice: `idx_file_ext(ext)`.

### 3.5 `file_fts` — FTS5 de archivos
Misma definición que `dir_fts`.

### 3.6 `dir_cache` — listados visitados
| Columna | Tipo | Notas |
|---|---|---|
| `path` | TEXT PK | Ruta canónica de la carpeta |
| `payload` | TEXT | JSON: `Vec<Entry>` |
| `bytes` | INTEGER | Tamaño de `payload` |
| `fetched_at` | TEXT | RFC 3339 |

**Política de expulsión**: tras cada inserción, si `SUM(bytes) > cache_max_mb` se borra el 25 % más antiguo por `fetched_at` (mínimo 1 fila).
**TTL de lectura**: `cache_ttl_hours` (12 por defecto); pasado el TTL se vuelve a listar.

### 3.7 Cambios en v2
Ninguno estructural. Se añade `source` ya existente en v1. *(Reservado: columna `proto` en `dir_cache` si se decide guardar con qué protocolo se leyó — no necesaria en 2.0.)*

## 4. `app.db`

### 4.1 `items` — contenido recordado
`path` PK, `name`, `kind`, `ext`, `section`, `year`, `size`, `first_seen`, `last_seen`.

### 4.2 `favorites`
| Columna | Tipo | Notas |
|---|---|---|
| `id` | INTEGER PK AUTOINCREMENT | |
| `target_kind` | TEXT | `CHECK IN ('item','folder')` |
| `path` | TEXT | Ruta canónica |
| `label` | TEXT NULL | Si está vacía se usa el nombre decodificado |
| `note` | TEXT NULL | |
| `created_at` | TEXT | |
| — | `UNIQUE(target_kind, path)` | |

### 4.3 `lists` y `list_items`
`lists(id, name, kind CHECK IN ('playlist','watchlist'), created_at, updated_at)`.
`list_items(list_id → lists ON DELETE CASCADE, path, name, position, added_at, watched_at NULL, PRIMARY KEY(list_id, path))`.

### 4.4 `downloads`
| Columna | Tipo | v2 |
|---|---|---|
| `id` | INTEGER PK | |
| `path` | TEXT | Ruta canónica remota |
| `dest` | TEXT | Ruta local final |
| `bytes_total`, `bytes_done` | INTEGER | |
| `state` | TEXT | `queued`, `active`, `paused`, `done`, `error`, `cancelled` |
| `started_at`, `finished_at`, `error` | TEXT | |
| **`proto`** | TEXT NULL | **Nuevo (v1→v2)**: `http` \| `ftp`, protocolo usado en el último intento |
| **`group_id`** | INTEGER NULL | **Nuevo**: agrupa los archivos de una descarga de carpeta |

Al arrancar, toda fila `active` o `queued` pasa a `paused` (la app se cerró durante la descarga).

### 4.5 `counters`
`path` PK, `attempts`, `completed`, `last_at`.

### 4.6 `searches`
`id`, `query`, `hits`, `at`. Se conservan las últimas 12 distintas para mostrar.

### 4.7 `settings`
`key` PK, `value` TEXT. Claves:

| Clave | Valores | Defecto | Origen |
|---|---|---|---|
| `bootstrap_ttl_hours` | entero | 24 | v1 |
| `cache_ttl_hours` | entero | 12 | v1 |
| `cache_max_mb` | entero | 64 | v1 |
| `index_mode` | `light` \| `deep` | `light` | v1 |
| `download_dir` | ruta | `<datos>/Downloads` | v1 |
| `player` | `auto` \| comando | `auto` | v1 |
| **`net_mode`** | `auto` \| `http` \| `ftp` | `auto` | v2 |
| **`proxy_mode`** | `system` \| `direct` \| `custom` | `system` | v2 |
| **`proxy_url`** | URL **sin contraseña** | vacío | v2 |
| **`proxy_user`** | texto | vacío | v2 |
| **`ftp_user`** | texto | `anonymous` | v2 |
| **`max_active_downloads`** | 1–3 | 1 | v2 |
| **`speed_limit_kbps`** | entero, 0 = sin límite | 0 | v2 |
| **`theme`** | `system` \| `dark` \| `light` | `dark` | v2 |
| **`ui_scale`** | 0.8–2.0 | 1.0 | v2 |
| **`reduce_motion`** | `system` \| `on` \| `off` | `system` | v2 |
| **`schema_notice_dismissed`** | `0`/`1` | `0` | v2 |

> **Contraseñas**: nunca en `settings`. Van al llavero del sistema bajo el servicio `com.novatic.novaflix` y la cuenta `proxy:<proxy_user>` o `ftp:<ftp_user>`.

## 5. Migración v1 → v2 (`migrate.rs`)

### 5.1 Algoritmo
1. Localizar el directorio de datos de v1 (mismo que v2).
2. Si existe `app.db` con `user_version = 0` y la tabla `items` (huella de v1):
   1. Copiar a `app.db.v1.bak` (si no existe ya).
   2. En una transacción:
      - `ALTER TABLE downloads ADD COLUMN proto TEXT`
      - `ALTER TABLE downloads ADD COLUMN group_id INTEGER`
      - `UPDATE downloads SET state='paused' WHERE state IN ('active','queued')`
      - `INSERT OR IGNORE INTO settings` con los valores por defecto nuevos.
      - `PRAGMA user_version = 1`
3. Para `cache.db`: si `user_version = 0` y las tablas coinciden, se adopta (`user_version = 1`); si algo falla, se **borra y reconstruye** (es desechable).
4. Cualquier error en el paso 2 revierte la transacción, se conserva `app.db.v1.bak` y se informa al usuario con su ruta (US-6.3).

### 5.2 Invariantes verificadas por pruebas
- El número de filas de `favorites`, `lists`, `list_items`, `counters` y `settings` es idéntico antes y después.
- Los `path` no cambian de forma (siguen siendo canónicos).
- Una segunda ejecución de la migración no cambia nada (idempotencia).
- Una base con `user_version` mayor al soportado no se modifica (`migrate::tests::future_versions_are_not_touched`).

## 6. Tipos en memoria (núcleo)

```rust
pub struct Entry   { name, path, kind: "dir"|"file", ext, size, mtime }   // listados
pub struct Hit     { kind, path, name, parent, section, year, ext, size, mtime, score }
pub struct SearchPage { hits, total, took_ms }
pub struct Listing { path, entries, fetched_at, cached, proto }            // proto nuevo
pub struct IndexStats { dirs, files, cache_bytes, cached_dirs, last_bootstrap, last_crawl, mode }
pub struct DownloadRow { id, path, dest, bytes_total, bytes_done, state, started_at, finished_at, error, proto, group_id }
pub struct Report  { proxy, dns, ftp, ftp_via_proxy, https, ftp_usable, http_usable, recommendation, summary }
```

Los tipos se serializan con `serde` solo donde se persisten (`Entry` en `dir_cache`) o se exportan (`Report` para «copiar diagnóstico»).

## 7. Reglas de integridad y seguridad de datos

- **R-D1** Las rutas siempre son canónicas; ninguna función pública acepta rutas locales.
- **R-D2** `dest` de una descarga se calcula con `sanitize_file_name(decode_name(name_of(path)))` y se une al directorio elegido; nunca se confía en el nombre remoto para componer rutas.
- **R-D3** Un `Report` serializado **nunca** contiene credenciales (probado).
- **R-D4** `settings` no almacena contraseñas (probado con una comprobación de claves permitidas).
- **R-D5** `cache.db` puede borrarse en cualquier momento sin pérdida de datos del usuario.
