# Changelog

Todas las versiones relevantes se documentan aquí. Formato basado en [Keep a Changelog](https://keepachangelog.com/es-ES/1.1.0/) y versionado [SemVer](https://semver.org/lang/es/).

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
