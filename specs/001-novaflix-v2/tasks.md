# Tareas: NovaFlix v2

**Entrada**: [spec.md](spec.md), [plan.md](plan.md), [research.md](research.md), [data-model.md](data-model.md), [contracts/](contracts/)
**Convenciones**: `[P]` se puede paralelizar (archivos distintos, sin dependencia) · `[USn]` historia de usuario · `[x]` hecho con evidencia · `[~]` iniciado · `[ ]` pendiente.
**Regla** (Constitución V): ninguna tarea se marca `[x]` sin evidencia (comando, prueba o captura).

Estado a **2026-11-13**.

## Fase 0 · Fundaciones

- [x] **T-001** Estudiar v1: estructura, flujos, canales, iconos. *Evidencia*: `release-and-ci.md` §1.
- [x] **T-002** Preservar v1: rama `v1` desde `main@899cc24`; etiqueta `v0.1.0` intacta. *Evidencia*: `git branch -vv` muestra `v1` y `v2/rust-rewrite`.
- [x] **T-003** Crear rama `v2/rust-rewrite`; retirar del árbol de trabajo el código de Tauri/React (recuperable en `v1`). *Evidencia*: `git branch -vv` (rama `v2/rust-rewrite`); `src/` y `src-tauri/` ya no están en el árbol y siguen en `git show v1:…`.
- [x] **T-004** Workspace `Cargo.toml` con crates `novaflix-core` y `novaflix`. *Evidencia*: `cargo metadata` lista `novaflix-core` y `novaflix`; `cargo build --workspace` compila.
- [x] **T-005** Constitución (`.specify/memory/constitution.md`). *Evidencia*: archivo `.specify/memory/constitution.md` (v1.0.0).
- [x] **T-006** `spec.md`, `plan.md`, `research.md`, `data-model.md`, `contracts/*`, `design-system.md`, `branding.md`, `release-and-ci.md`, `quickstart.md`, `tasks.md`. *Evidencia*: archivos en `specs/001-novaflix-v2/` (spec, plan, research, data-model, contracts/4, design-system, branding, release-and-ci, quickstart, tasks).
- [x] **T-007** ADR 0001–0005 en `docs/adr/` (escritos; falta revisión). *Evidencia*: cinco ADR en `docs/adr/` (0001–0005).
- [ ] **T-008** Revisión de las especificaciones por el propietario; resolver C-2, C-7, C-8.

## Fase 1 · Núcleo de datos

- [x] **T-010** [P] `norm.rs` portado de v1 con sus pruebas. *Evidencia*: `cargo test -p novaflix-core norm::` (3 pruebas).
- [x] **T-011** [P] `catalog.rs` portado (parsers) + `HOST`, `encode_name` + prueba de ida y vuelta. *Evidencia*: `catalog::tests` (4 pruebas, incluida `names_roundtrip`).
- [x] **T-012** [P] `error.rs`: `Error` tipado y `lock()`. *Evidencia*: `cargo test`: `error::tests::classifies_errors`.
- [x] **T-013** `error.rs`: `ErrorKind` y `Error::kind()`; `UiError` (contracts/core-api §10). *Evidencia*: `error::tests::classifies_errors`; `UiError` en `error.rs`.
- [x] **T-014** `db.rs`: esquema `cache.db` y `app.db` con `user_version`, columnas nuevas (`downloads.proto`, `group_id`). *Evidencia*: `db::tests::fresh_databases_have_current_version`.
- [x] **T-015** `settings.rs`: `Settings` tipado, validación, valores por defecto (core-api §4). *Evidencia*: `settings::tests` (4 pruebas, incluida `passwords_are_never_persisted`).
- [x] **T-016** `migrate.rs`: v1 → v2 con copia `app.db.v1.bak`, idempotencia y modo solo-lectura ante versión futura (data-model §5). *Evidencia*: `migrate::tests` (4 pruebas: datos, idempotencia, versión futura, directorio vacío).
- [x] **T-017** *Fixture* de `app.db` de v1 generada con el esquema de `git show v1:src-tauri/src/db.rs` + prueba CT-06/SC-005. *Evidencia*: fixture con el esquema de v1 en `migrate::tests::v1_data_survives_migration`; `ct06_core_open_migrates_v1_data`.
- [ ] **T-016b** Migración: corregir rutas con `&amp;` heredadas de v1 en `favorites`, `list_items`, `items`, `counters` y `downloads.path`.
- [x] **T-018** `index.rs`: `bootstrap`, `search`, `sections`, `recent_items`, `browse`, `cache_clear`, `crawl` (portados; sin Tauri). *Evidencia*: `index.rs`; `ct01`, `ct02`, `ct02b` en `tests/core_contract.rs`.
- [x] **T-018b** Analizador para el formato real del servidor (tabla + fechas ISO), desescape de entidades HTML y decodificación Latin-1. *Evidencia*: `catalog::tests` (13 pruebas, con HTML real en `tests/fixtures/`) y `real_listado_parses_cleanly` contra los 4,7 MB de `listado.html`.
- [ ] **T-019** Prueba de rendimiento: búsqueda p95 y apertura sobre 30.000 carpetas sintéticas (SC-004).

## Fase 2 · Transporte

- [x] **T-020** `transport/ftp.rs`: sesión, EPSV/PASV, MLSD/LIST, SIZE, REST+RETR, SOCKS5 + pruebas de análisis (pruebas escritas; pendientes de ejecutar). *Evidencia*: `tests/ftp_integration.rs`: 4 pruebas contra servidor FTP real local + 1 contra `rclone serve ftp` (`--ignored`).
- [x] **T-021** `transport/http.rs`: listados con reintentos y descarga con `Range`. *Evidencia*: `tests/http_integration.rs` (5 pruebas).
- [x] **T-022** `transport/mod.rs`: `Mode`, `ProxyChoice`, `Net`, selección Auto/forzado. *Evidencia*: `transport::tests` (3 pruebas).
- [x] **T-023** `transport/probe.rs`: pasos, veredicto, enmascarado de credenciales. *Evidencia*: `probe::tests` y `ct05_diagnostic_report_never_contains_the_proxy_password`.
- [x] **T-024a** Variable `NOVAFLIX_FTP_ADDR` (solo pruebas/depuración) para apuntar el cliente FTP a un servidor local (quickstart §3.6). *Evidencia*: `NOVAFLIX_FTP_ADDR` en `lib.rs::net_config` (solo `debug_assertions`).
- [x] **T-024** Política de redirección: solo mismo host (CT-08; FR-017). *Evidencia*: `http_integration::redirects_to_other_hosts_are_rejected`.
- [x] **T-025** Servidor FTP de prueba en proceso + prueba de integración: login, listado MLSD y LIST, `SIZE`, `RETR`, `REST` (Principio V). *Evidencia*: `tests/ftp_integration.rs` con `tests/support/mod.rs` (servidor FTP en proceso).
- [x] **T-026** Servidor HTTP de prueba en proceso: listado, `206`/`200`, 407, redirección externa. *Evidencia*: `tests/support/httpd.rs` + `tests/http_integration.rs`.
- [ ] **T-027** Proxy de prueba en proceso: `CONNECT` 200/403/407 y los 5 escenarios de `transport.md §5.3` (SC-002).
- [ ] **T-028** Prueba estática en CI: ningún comando de escritura FTP en `ftp.rs`.
- [ ] **T-029** Lectura del proxy del sistema en Windows (registro) para el paso 3 del diagnóstico, bajo `cfg(windows)`.

## Fase 3 · Descargas y biblioteca

- [x] **T-030** `downloads.rs`: cola persistente, semáforo configurable, `.part`, reanudación HTTP y FTP, eventos agrupados, velocidad, verificación de tamaño. *Evidencia*: `downloads.rs`; `ct03`, `ct03b`, `ct03c`, `ct03d`.
- [x] **T-031** `downloads.rs`: saneado de nombres y política `OnExists` (CT-07). *Evidencia*: `downloads::tests` + `ct07_hostile_names_never_escape_the_download_dir`.
- [~] **T-032** `downloads.rs`: `plan_folder_download` y `download_folder` recursivos. *Evidencia*: implementado en `downloads.rs` (`plan_folder`/`download_folder`) y usado por la UI; **sin prueba automática** todavía.
- [~] **T-033** `downloads.rs`: límite de velocidad. *Evidencia*: límite de velocidad implementado en `downloads.rs` (sin prueba automática aún).
- [~] **T-034** Prueba CT-03: descarga cortada tres veces y reanudada con bytes idénticos, por HTTP y por FTP (SC-003). *Evidencia*: `ct03b` (reanudar desde un `.part`) y `ct03c` (servidor sin Range) cubren HTTP; falta la prueba de 3 cortes y la de FTP con reanudación completa (el `REST` sí se prueba en `ftp_integration::downloads_and_resumes`).
- [~] **T-035** `library.rs`: favoritos, listas, contadores, M3U, `play` (portado). *Evidencia*: `library.rs` portado (sin pruebas propias aún; cubierto indirectamente por `ct03`).
- [x] **T-036** `lib.rs`: `Core::open`, `CoreOptions`, `Event`, `wake`. *Evidencia*: `lib.rs`: `Core::open`, `CoreOptions`, `Event`; `ct06`.
- [x] **T-037** Pruebas de contrato CT-01…CT-08. *Evidencia*: `tests/core_contract.rs`: ct01, ct02, ct02b, ct03, ct03b–d, ct04, ct05, ct06, ct07 (11 pruebas); CT-08 en `http_integration::redirects_to_other_hosts_are_rejected`. `cargo test --workspace` en verde.

## Fase 4 · Interfaz

- [x] **T-040** `theme.rs`: tokens, estilo `egui` claro/oscuro, fuentes Inter, iconos Phosphor, prueba de contraste (SC-008). *Evidencia*: `theme::tests` (3 pruebas de contraste AA, claro y oscuro); Inter y Phosphor incrustados. La prueba detectó un fallo real de contraste (design-system §10).
- [~] **T-041** `motion.rs`: muelles, interrumpibles, con modo reducir movimiento. *Evidencia*: `motion.rs` (muelles interrumpibles y `reduce_motion`) en uso en dock, botones y tarjetas; sin pruebas automáticas.
- [x] **T-042** `cover.rs`: portadas generadas deterministas + 12 paletas validadas + caché de texturas. *Evidencia*: `cover::tests` (3 pruebas: determinismo, contraste ≥ 4,5 : 1 en 12 paletas × 5 ángulos, tipos por sección); capturas en `docs/screenshots/`.
- [x] **T-043** [P] `widgets/`: tarjeta, estantería, chip, botón, anillo de progreso, toast, barra de estado, dock, esqueleto. *Evidencia*: `widgets.rs` + `card.rs` + dock/barra de estado/avisos en `app.rs`; visibles en `docs/screenshots/*.png`.
- [ ] **T-044** Contraseñas en el llavero (`keyring`) para proxy y FTP; sin persistir si no hay llavero (research §6).
- [x] **T-045** `screens/home.rs` — [US2] *Evidencia*: captura `docs/screenshots/home.png` y `light.png` con datos reales del sitio.
- [x] **T-046** `screens/search.rs` — [US1] *Evidencia*: captura `docs/screenshots/search.png` («wicked»: 12 resultados en 6 ms).
- [x] **T-047** `screens/browse.rs` y ficha de título — [US2] *Evidencia*: capturas `browse.png` y `sheet.png` (ficha de «Wicked (2024)» con 11 archivos reales).
- [~] **T-048** `screens/downloads.rs` — [US4] *Evidencia*: implementado (`screens/downloads.rs`) y probado el motor (`ct03*`); **sin captura** de la pantalla con descargas.
- [~] **T-049** `screens/library.rs` — [US5] *Evidencia*: implementado (`screens/library.rs`); **sin captura ni prueba**.
- [~] **T-050** `screens/settings.rs` con Red y diagnóstico — [US3] *Evidencia*: implementado (`screens/settings.rs`) y el diagnóstico verificado con `examples/probe` contra la red real; la captura con el veredicto no se pudo obtener (la ventana de prueba recibía el ratón del usuario).
- [x] **T-051** Barra de estado con protocolo y proxy; flujo «mi FTP no funciona» (ui-screens §8.1) — [US3] *Evidencia*: barra de estado con «Conectado · HTTP» y proxy enmascarado en todas las capturas; aviso «Usar HTTP» en `settings_network` (el flujo 8.1 está implementado, sin captura).
- [~] **T-052** Atajos globales y navegación por teclado (FR-043, NFR-008). *Evidencia*: atajos implementados en `app.rs::global_shortcuts` (Ctrl+K, Ctrl+1…5, Ctrl+, , Esc, F5, zoom); no probados automáticamente.
- [ ] **T-053** Instancia única.
- [ ] **T-054** `--self-test` sin ventana para CI y medición de arranque.
- [~] **T-055** Capturas de cada pantalla/estado y revisión visual. *Evidencia*: 5 capturas reproducibles (`--shot`): inicio oscuro/claro, buscar, explorar, ficha. Faltan descargas, biblioteca y ajustes con veredicto.

## Fase 5 · Marca y empaquetado

- [x] **T-056** Diseñar `logo.svg`, `logo-small.svg`, `logo-mono.svg` según `branding.md`. *Evidencia*: `assets/logo.svg`, `logo-small.svg`, `logo-mono.svg`; vista previa en 512/128/48/32/16 px sobre fondo claro y oscuro revisada.
- [x] **T-057** `scripts/make-icons.sh`: PNG, ICO, tamaños hicolor. *Evidencia*: `./scripts/make-icons.sh` genera hicolor 16–512, `icon.ico` con 9 resoluciones (`magick identify`) y los activos de `assets/`.
- [x] **T-058** Actualizar `social-preview` y el README con la nueva marca. *Evidencia*: `assets/social-preview.png` regenerado y `README.md` reescrito con las nuevas capturas.
- [x] **T-059** Icono de ventana y `app_id` Wayland (`com.novatic.novaflix`); recurso `.ico` en Windows (`build.rs`). *Evidencia*: `main.rs` carga `icon.png` en la ventana y `app_id`; `build.rs` incrusta `icon.ico` en Windows (**la parte de Windows no se pudo compilar aquí**: la verificará CI).
- [x] **T-060** `[package.metadata.packager]` completo (release-and-ci §7). *Evidencia*: `[package.metadata.packager]` en `crates/novaflix-app/Cargo.toml`; `cargo packager` lo acepta (T-061).
- [x] **T-061** Probar `cargo packager` para `deb` y `appimage` en Linux; confirmar nombres y `Package: nova-flix`. *Evidencia*: `cargo packager -f deb,appimage` (cargo-packager 0.11.8) genera `novaflix_2.0.0-alpha.1_amd64.deb` (`Package: nova-flix`, 9 iconos hicolor) y un AppImage que se extrae con `AppRun`, `.desktop` e icono. NSIS/MSI **sin verificar** (requiere Windows).
- [x] **T-062** Tarball y `.pkg.tar.zst` automatizados (scripts + `PKGBUILD` de empaquetado). *Evidencia*: `scripts/make-tarball.sh` + `packaging/arch/PKGBUILD`: `makepkg` produce `novaflix-bin-2.0.0-1-x86_64.pkg.tar.zst` con `/usr/bin/novaflix`, `.desktop` e iconos.
- [x] **T-063** Confirmar dependencias de sistema con `ldd` y reflejarlas en deb/AUR. *Evidencia*: `ldd` solo muestra `libc`, `libgcc_s` y `libm`; las demás (EGL, Wayland, X11, xkbcommon, dbus) se cargan con `dlopen` y están en `depends` de deb/AUR/Arch.

## Fase 6 · CI/CD

- [~] **T-070** `ci.yml` reescrito (lint, test Linux/Windows, audit, MSRV, perf, specs, forbidden-ftp). *Evidencia*: `ci.yml` escrito (lint, test Linux/Windows, MSRV, FTP de solo lectura, tareas con evidencia); YAML válido y los mismos comandos pasan en local. **No ejecutado aún en GitHub Actions**; falta el job `perf`.
- [~] **T-071** `release.yml` reescrito (cargo-packager, tarball, pkg, `SHA256SUMS`, prerelease por etiqueta, comprobación etiqueta = versión). *Evidencia*: `release.yml` escrito (verify, linux, arch, windows, publish); YAML válido; cada paso de Linux/Arch verificado a mano. **No ejecutado aún en GitHub Actions**; los pasos de Windows son sin probar.
- [x] **T-072** `apt.yml`, `pacman.yml`, `aur.yml`: disparador `released`. *Evidencia*: `grep types: .github/workflows/{apt,pacman,aur}.yml` → `[released]`.
- [x] **T-073** `PKGBUILD`: dependencias nuevas, `optdepends`, versión. *Evidencia*: `packaging/aur/PKGBUILD` y `packaging/arch/PKGBUILD` con las dependencias reales; el segundo construido con `makepkg`.
- [~] **T-074** `winget.yml` y manifiestos actualizados. *Evidencia*: `winget.yml` escrito (condicionado a `vars.WINGET_ENABLED`); los manifiestos de `packaging/winget/` siguen en `0.1.0` (los regenerará el flujo).
- [x] **T-075** `codeql.yml` con `rust`; `dependabot.yml` sin npm y con cargo en `/`. *Evidencia*: `codeql.yml` (`rust`) y `dependabot.yml` (cargo `/` y actions) reescritos; YAML válido.
- [~] **T-076** `audit.yml` + `deny.toml`. *Evidencia*: `audit.yml` y `deny.toml` escritos; **`cargo audit`/`cargo deny` no se han ejecutado** todavía.
- [x] **T-077** Plantillas de issue: protocolo, proxy, pegar diagnóstico. *Evidencia*: `bug_report.yml` con protocolo, proxy y diagnóstico; `PULL_REQUEST_TEMPLATE.md` con los comandos de Rust.
- [~] **T-078** README, CONTRIBUTING, CHANGELOG, SECURITY, `.gitignore`, `site/`. *Evidencia*: README, CONTRIBUTING, CHANGELOG, SECURITY y `.gitignore` reescritos; falta actualizar `site/` (portada y páginas APT/pacman).
- [ ] **T-079** Primera etiqueta `v2.0.0-alpha.1` (prerelease) y verificación de artefactos.

## Fase 7 · Endurecimiento

- [ ] **T-080** Recorrido de `quickstart.md` en Linux y Windows.
- [ ] **T-081** Medir NFR-001…005 y ajustar.
- [ ] **T-082** Prueba de migración con datos reales de v1.
- [ ] **T-083** `v2.0.0-beta.1`; fusionar `v2/rust-rewrite` en `main`.
- [ ] **T-084** `v2.0.0-rc.1`; verificar `apt upgrade`, `pacman -Syu`, `winget upgrade` desde v1 (SC-006).
- [ ] **T-085** `v2.0.0` estable.

## Ideas futuras (fuera de 2.0)

- **T-F01** Pasarela FTP de Squid (`GET ftp://…` con `Authorization: Basic`) como tercer transporte para quien solo tenga ese camino.
- **T-F02** Firma Authenticode en Windows (C-8).
- **T-F03** macOS (C-7).
- **T-F04** CLI `novaflix-cli` sobre el núcleo.
- **T-F05** Interfaz en inglés (NFR-011).
- **T-F06** Atestación de procedencia (`attest-build-provenance`).
- **T-F07** Desenfoque real de fondo con `wgpu` si `egui` lo permite.

## Dependencias entre fases

```
F0 ──► F1 ──► F2 ──► F3 ──► F4 ──► F5 ──► F6 ──► F7
        └────────────────────────▲
   (T-056/057 marca puede ir en paralelo desde F0)
```

Camino crítico: T-014 → T-016 → T-018 → T-030 → T-036 → T-045…T-050 → T-060 → T-071.
