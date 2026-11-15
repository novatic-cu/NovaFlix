# Contribuir a NovaFlix

Gracias por tomarte el tiempo. Este documento resume cómo preparar el entorno, qué estilo seguir y qué esperar de una revisión.

## Cómo trabajamos: Spec-Driven Development

Antes del código van las especificaciones. Todo está en el repositorio:

| Documento | Para qué |
|---|---|
| [`.specify/memory/constitution.md`](.specify/memory/constitution.md) | Principios que no se negocian (privacidad, red honesta, solo lectura, pruebas, seguridad…) |
| [`specs/001-novaflix-v2/spec.md`](specs/001-novaflix-v2/spec.md) | Qué hace la v2 y por qué (historias, requisitos, criterios de éxito) |
| [`plan.md`](specs/001-novaflix-v2/plan.md), [`research.md`](specs/001-novaflix-v2/research.md) | Cómo se construye y qué alternativas se descartaron |
| [`data-model.md`](specs/001-novaflix-v2/data-model.md), [`contracts/`](specs/001-novaflix-v2/contracts/) | Esquema de datos y contratos del núcleo, eventos, transporte y pantallas |
| [`design-system.md`](specs/001-novaflix-v2/design-system.md), [`branding.md`](specs/001-novaflix-v2/branding.md) | Interfaz e icono |
| [`release-and-ci.md`](specs/001-novaflix-v2/release-and-ci.md) | Ramas, versiones, CI/CD y canales |
| [`tasks.md`](specs/001-novaflix-v2/tasks.md) | Tareas con estado y **evidencia** |
| [`docs/adr/`](docs/adr/) | Decisiones de arquitectura |

Reglas prácticas:

1. Un cambio de comportamiento empieza por la especificación (o su contrato) y termina con la tarea marcada en `tasks.md` **con evidencia** (prueba, comando o captura). CI lo comprueba (`scripts/check-tasks.sh`).
2. Si una decisión contradice la constitución, se propone primero una enmienda.
3. Las decisiones con alternativas reales llevan un ADR.

## Entorno

```bash
cargo run -p novaflix
cargo test --workspace
```

Requisitos:

- Rust ≥ 1.95 (`rustup`).
- Linux: bibliotecas de desarrollo de X11/Wayland y OpenGL (`libxkbcommon-dev`, `libwayland-dev`, `libgl1-mesa-dev`, `libegl1-mesa-dev`, `pkg-config`).
- Windows: MSVC Build Tools. No hace falta WebView2.

Con `NOVAFLIX_DATA_DIR=/ruta/temporal` la aplicación usa una base de datos aislada (útil para no tocar tus datos).

## Flujo de trabajo

1. Abre un issue describiendo el problema o la propuesta antes de cambios grandes.
2. Crea una rama: `feat/<tema>`, `fix/<tema>`, `docs/<tema>`. La reescritura v2 vive en `v2/rust-rewrite`; la v1 se mantiene en `v1`.
3. Commits en estilo [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`.
4. Abre un pull request con una descripción clara y capturas si toca la interfaz.

## Estándares de código

### Rust

- `cargo fmt --all` y `cargo clippy --workspace --all-targets -- -D warnings` deben pasar.
- La lógica va en `novaflix-core`; la interfaz (`novaflix-app`) solo pinta estado y envía órdenes (Constitución II).
- Errores tipados (`novaflix_core::Error`); sin `unwrap()`/`expect()` en rutas de usuario.
- La red nunca bloquea el hilo de la interfaz: `tokio` en el núcleo y eventos hacia la UI.
- El cliente FTP **no puede** emitir comandos de escritura (`scripts/check-ftp-readonly.sh` lo vigila).
- Los textos que ve el usuario están en español y explican qué pasó y qué hacer (Constitución III).

### Interfaz (egui)

- Sin colores literales fuera de `theme.rs`; usa los tokens de «Nova Glass» ([`design-system.md`](specs/001-novaflix-v2/design-system.md)).
- Una acción primaria por pantalla; estados completos (cargando, vacío, error, éxito).
- Movimiento con muelles interrumpibles (`motion.rs`) y desactivable con «reducir movimiento».
- Contraste AA: `theme::tests` lo verifica; si añades un color, añádelo a la prueba.
- Capturas reproducibles: `novaflix --screen search --query wicked --shot out.png` (cierra la ventana al terminar). Úsalas en los PR de interfaz.

## Pruebas

```bash
cargo test --workspace                          # unitarias, integración (FTP/HTTP locales) y contrato
NOVAFLIX_TEST_FTP=127.0.0.1:2121 cargo test -p novaflix-core --test ftp_integration -- --ignored
                                                # contra un servidor FTP independiente (p. ej. rclone serve ftp)
NOVAFLIX_REAL_LISTADO=/ruta/listado.html cargo test -p novaflix-core --lib -- --ignored real_listado
cargo run -p novaflix-core --example smoke      # sitio real: índice, búsqueda, listado y descarga
cargo run -p novaflix-core --example probe      # diagnóstico de red
```

- Si añades parsers, normalización, esquema o transporte, incluye pruebas. Para los analizadores usa **HTML o respuestas reales** (hay fixtures en `crates/novaflix-core/tests/fixtures/`): el servidor ya cambió de formato una vez.
- El FTP se prueba contra un servidor FTP real (`tests/support/`), no solo contra texto simulado.

## Iconos

`scripts/make-icons.sh` regenera todos los derivados (PNG, ICO, hicolor) desde `assets/logo*.svg`. Requiere `rsvg-convert` e ImageMagick (`magick`). Los derivados se versionan.

## Licencia de las contribuciones

Al enviar un pull request aceptas que tu contribución se publique bajo la licencia del proyecto (PolyForm Noncommercial 1.0.0) y, si procede, bajo la licencia comercial descrita en [COMMERCIAL.md](COMMERCIAL.md).
