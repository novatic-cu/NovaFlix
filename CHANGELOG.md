# Changelog

Todas las versiones relevantes se documentan aquí. Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y versionado [SemVer](https://semver.org/lang/es/).

## [2.0.0-alpha.2] — 2026-11-14

### Añadido

- Pósters reales descubiertos en la carpeta de cada título (con caché en disco y resolución episodio → serie); esqueletos mientras cargan.
- Inicio de altura fija: portada, categorías en chips, Novedades en tantas filas como quepan y **Descubre** (contenido al azar con *scroll* propio); cada zona desplaza solo bajo el cursor.
- Workflow `site.yml` que publica `site/` en GitHub Pages sin tocar los repositorios APT/pacman.

### Cambiado

- Sitio web y README reescritos para la v2 (alpha, descargas desde Releases, solo Linux y Windows).
- Los releases alpha no generan `.msi` (WiX no admite versiones con pre-release alfanumérica); solo NSIS.

### Corregido

- **Seguridad**: `rusqlite` 0.42 (SQLite 3.54) corrige RUSTSEC-2026-0459.
- CI: permisos de `cargo-audit` y licencias permitidas en `cargo-deny`.
- Las capturas de `docs/` ya no muestran la barra de estado con datos del proxy.

## [2.0.0-alpha.1] — 2026-11-14

Reescritura completa en Rust (la versión 1 queda en la rama `v1`).

### Añadido

- Aplicación nativa en Rust (`egui`) para Linux y Windows, sin WebView2 ni WebKitGTK.
- Interfaz de aplicación de streaming («Nova Glass»): portada con novedades, estanterías, categorías, ficha por título, descargas con anillos de progreso, tema oscuro/claro, atajos de teclado y «reducir movimiento».
- Portadas generadas de forma determinista, sin servicios externos.
- Transporte **HTTP o FTP a elección** (Automático, HTTP, FTP). Cliente FTP propio, de solo lectura (`MLSD`/`LIST`, `SIZE`, `REST`, `RETR`) con soporte de proxy SOCKS5.
- **Diagnóstico de red**: DNS, puerto 21, túnel `CONNECT` por el proxy y HTTPS, con un veredicto en lenguaje llano; en modo Automático se elige HTTP cuando FTP no es posible.
- Descargas con cola persistente, `.part`, reanudación (`Range`/`REST`), verificación de tamaño, límite de velocidad y descarga recursiva de carpetas.
- Migración automática de los datos de la versión 1, con copia `app.db.v1.bak`.
- Empaquetado con `cargo-packager` (`.deb`, `.AppImage`, NSIS, MSI), `.tar.gz` y paquete Arch generados por CI, y `SHA256SUMS`.
- Documentación Spec-Driven Development (`.specify/`, `specs/`, `docs/adr/`).
- Nuevo icono.

### Cambiado

- El analizador de listados reconoce el formato actual del servidor (tabla de Apache con fechas ISO) además del anterior.
- Las publicaciones en APT, pacman y AUR solo se disparan con versiones estables (`released`), no con prereleases.
- CI en Linux y Windows (formato, clippy, pruebas, MSRV, auditoría de dependencias, CodeQL para Rust).

### Corregido

- Las rutas con `&` ya no se guardan escapadas (`&amp;`); la migración repara las de la versión 1.
- `listado.html` con codificación mixta (UTF-8/Latin-1) ya no rompe el índice.

### Eliminado

- Soporte de macOS en esta serie (la versión 1 sigue disponible).

## [0.1.0] — 2026-10-27

### Añadido

- Buscador de escritorio (Tauri 2 + React 19) para catálogos multimedia públicos.
- Índice local SQLite/FTS5 con bootstrap del árbol publicado en ~1 s.
- Normalización leet (`Wick3d` → `wicked`), prefijos y filtros por sección, año y extensión.
- Explorador con listados en vivo, caché TTL y LRU configurable.
- Favoritos por contenido y por carpeta.
- Playlists ordenables y lista de pendientes con estado *visto*.
- Descargas con cola, reanudación por `Range`, progreso en vivo y contadores por ruta.
- Exportación M3U/M3U8 y reproducción con reproductor externo.
- Modo de índice profundo (crawl BFS) con progreso en segundo plano.
- Empaquetado `.deb`, `.AppImage`, `.pkg.tar.zst` (Arch), NSIS, MSI y DMG.
- Tema claro/oscuro, soporte de `prefers-reduced-motion`, `prefers-reduced-transparency` y `prefers-contrast`.

### Seguridad

- CSP estricta, allowlist de host, TLS verificado y saneado de nombres en descargas.
- Sin telemetría ni cuentas.
