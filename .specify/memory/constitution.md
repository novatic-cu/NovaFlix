# Constitución de NovaFlix

**Versión**: 1.0.0 · **Ratificada**: 2026-11-13 · **Última enmienda**: 2026-11-13

Este documento fija los principios que gobiernan cualquier especificación, plan o cambio de código de NovaFlix. Una especificación o un plan que contradiga un principio **no se aprueba** hasta que se enmiende la constitución (ver *Gobierno*). Los términos MUST / SHOULD / MAY se usan en el sentido de RFC 2119.

## Contexto

NovaFlix es una aplicación de escritorio para explorar, buscar y descargar contenido de catálogos multimedia públicos, hoy centrada en `visuales.uclv.cu`. La v1 (rama `v1`, etiqueta `v0.1.0`) está hecha con Tauri 2 + React 19. La v2 es una reescritura íntegra en Rust, para Windows y Linux, con una interfaz de aplicación de streaming.

## Principios

### I. Privacidad por diseño

- La aplicación **MUST NOT** exigir cuentas, **MUST NOT** enviar telemetría ni analítica, y **MUST NOT** contactar a ningún host distinto del catálogo configurado (y, en su caso, del proxy elegido por el usuario).
- Los secretos (contraseñas de proxy o de FTP) **MUST NOT** escribirse en registros, mensajes de error, capturas de diagnóstico ni en la base de datos en texto plano. Se guardan en el llavero del sistema o no se guardan.
- Todo texto que muestre una URL de proxy **MUST** ocultar la contraseña.

*Razón*: v1 ya prometía «sin cuentas, sin telemetría». Es parte de la identidad del producto y de la confianza de quien lo instala.

### II. Núcleo primero, interfaz delgada

- Toda la lógica (catálogo, caché, búsqueda, descargas, transporte, diagnóstico, biblioteca) **MUST** vivir en el crate `novaflix-core`, sin depender de ningún toolkit gráfico.
- La interfaz **MUST** limitarse a pintar estado y enviar órdenes al núcleo. Cualquier regla de negocio hallada en la UI es un defecto.
- El núcleo **MUST** poder ejercitarse sin interfaz (pruebas, y en el futuro una CLI).

*Razón*: es lo que permitió portar v1 a Rust sin reescribir el comportamiento, y lo que hace probable una CLI o un cambio de toolkit.

### III. Red honesta

- Si algo falla por la red, la aplicación **MUST** decir qué falló y por qué, en lenguaje llano, y proponer la salida (por ejemplo: «tu proxy bloquea FTP; se usa HTTP»).
- Un fallo de red **MUST NOT** presentarse como una pantalla vacía, un temporizador sin explicación o un error técnico crudo.
- El usuario **MUST** poder elegir el protocolo (HTTP, FTP o automático) y ver cuál se está usando.
- La aplicación **MUST** funcionar detrás de un proxy corporativo con autenticación sin configuración manual en el caso común (proxy del sistema).

*Razón*: el origen de este proyecto fue un cliente FTP que daba «timeout» sin explicación detrás de un proxy Squid que respondía 403 al puerto 21.

### IV. Solo lectura y respeto al servidor

- NovaFlix **MUST** ser un cliente de solo lectura: sin subidas, borrados ni modificaciones remotas.
- La aplicación **MUST** limitar la concurrencia contra el servidor (por defecto, 1 descarga activa y 6 listados simultáneos en un rastreo), **MUST** reintentar con espera creciente y **SHOULD** cachear listados con TTL para no repetir peticiones.
- NovaFlix **MUST NOT** alojar ni redistribuir contenido; la descarga es responsabilidad de quien la ordena.

### V. Verificar antes de afirmar (test-first donde importa)

- Los analizadores (índice de Apache, MLSD, LIST), la normalización de nombres, el esquema de datos y el transporte **MUST** tener pruebas automáticas, escritas antes o junto con el código.
- El transporte FTP **MUST** probarse contra un servidor FTP real (local, efímero) y no solo contra respuestas simuladas.
- La integración continua **MUST** ejecutar `cargo fmt --check`, `cargo clippy -D warnings` y `cargo test` en Linux y Windows. Un cambio que rompa alguno **MUST NOT** fusionarse.
- Ninguna tarea se marca como hecha en `tasks.md` sin evidencia (prueba, captura o comando reproducible).

### VI. Seguridad por defecto

- TLS **MUST** verificarse siempre; no existe opción para desactivar la verificación.
- Los nombres de archivo remotos **MUST** sanearse antes de escribir en disco (sin separadores, sin rutas relativas, sin caracteres de control) y las descargas **MUST** escribirse a `.part` y renombrarse al terminar.
- El host del catálogo **MUST** estar en una lista de permitidos en el núcleo; una redirección a otro host **MUST** rechazarse.
- Las dependencias **MUST** auditarse en CI (`cargo audit`/`cargo deny`) y mantenerse con Dependabot; el código **SHOULD** analizarse con CodeQL.

### VII. Rendimiento y accesibilidad como requisitos, no como pulido

- Arranque en frío hasta pantalla útil **SHOULD** ser < 1,5 s con caché caliente; la búsqueda **SHOULD** responder en < 50 ms sobre 30.000 carpetas.
- La interfaz **MUST** mantener 60 fps al desplazarse por estanterías y rejillas, y **MUST NOT** bloquear el hilo de UI con red, disco ni SQLite pesado.
- La interfaz **MUST** ser operable solo con teclado, respetar el contraste mínimo WCAG AA (4,5:1 en texto), escalar con el zoom del sistema y respetar «reducir movimiento».

### VIII. Distribución reproducible y verificable

- Todo artefacto publicado (instaladores, repositorios APT/pacman, manifiestos) **MUST** salir de CI a partir de una etiqueta `vX.Y.Z`; nada se sube a mano.
- Cada release **MUST** incluir sumas SHA-256 y los repositorios APT/pacman **MUST** seguir firmados con la clave publicada.
- El versionado **MUST** seguir SemVer; el historial de cambios **MUST** mantenerse en `CHANGELOG.md` (Keep a Changelog).

### IX. Continuidad con v1

- Los datos de usuario de v1 (favoritos, listas, contadores, ajustes, descargas) **MUST** migrarse automáticamente y sin pérdida a v2.
- Los nombres públicos (`NovaFlix`, paquete `novaflix`, paquete APT `nova-flix`, identificador `com.novatic.novaflix`, repositorio `novatic-cu/NovaFlix`) **MUST** conservarse para que las instalaciones existentes se actualicen en lugar de duplicarse.
- La v1 **MUST** permanecer recuperable (rama `v1` y etiqueta `v0.1.0`).

## Restricciones técnicas

| Ámbito | Decisión | Documento |
|---|---|---|
| Lenguaje | Rust estable (edición 2021, MSRV 1.95) | ADR-0002 |
| Plataformas | Linux x86_64 y Windows x86_64 | spec.md |
| Interfaz | `egui`/`eframe`, sin webview | ADR-0002 |
| Persistencia | SQLite (`rusqlite`, bundled) | data-model.md |
| Red | `reqwest` + `rustls` para HTTP; cliente FTP propio sobre `tokio` | ADR-0003 |
| Licencia | PolyForm Noncommercial 1.0.0 | LICENSE |

## Gobierno

1. **Enmiendas**: se proponen por pull request que modifique este archivo, incrementando la versión (MAJOR si se elimina o redefine un principio; MINOR si se añade uno o se amplía sustancialmente; PATCH si solo se aclara).
2. **Cumplimiento**: cada `plan.md` incluye una sección *Constitution Check*; cada revisión de código verifica los principios I, III, V y VI.
3. **Excepciones**: una desviación puntual se documenta en la sección *Complexity Tracking* del plan, con su justificación y la alternativa más simple descartada.
