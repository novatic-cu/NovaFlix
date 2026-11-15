# Plan de implementación: NovaFlix v2

**Rama**: `v2/rust-rewrite` · **Fecha**: 2026-11-13 · **Spec**: [spec.md](spec.md)
**Entrada**: especificación de funcionalidad aprobada.
**Documentos hermanos**: [research.md](research.md) · [data-model.md](data-model.md) · [contracts/](contracts/) · [design-system.md](design-system.md) · [branding.md](branding.md) · [release-and-ci.md](release-and-ci.md) · [quickstart.md](quickstart.md) · [tasks.md](tasks.md)

## 1. Resumen técnico

Reescritura íntegra de NovaFlix en un *workspace* de Rust con dos crates:

- **`novaflix-core`** (biblioteca): catálogo, normalización, SQLite/FTS5, índice, transporte HTTP/FTP, diagnóstico de red, cola de descargas, biblioteca personal, migración desde v1. Sin dependencias gráficas.
- **`novaflix`** (binario, `crates/novaflix-app`): interfaz `egui`/`eframe` con un sistema de diseño propio («Nova Glass»), portadas generadas, atajos y accesibilidad. Delgada: pinta estado y envía órdenes.

El núcleo se comunica con la interfaz por **órdenes** (llamadas a métodos de `Core`) y **eventos** (canal `tokio::sync::mpsc` + función de despertar la UI). Las operaciones de red se ejecutan en un *runtime* de `tokio` propiedad de la aplicación; las consultas SQLite rápidas (búsqueda, biblioteca) se llaman de forma síncrona; las lentas se ejecutan en `spawn_blocking`.

## 2. Contexto técnico

| Aspecto | Valor |
|---|---|
| Lenguaje / versión | Rust estable, edición 2021, MSRV 1.95 (toolchain de desarrollo: 1.99.1) |
| Dependencias principales | `tokio`, `reqwest` (rustls, system-proxy, gzip, stream, socks), `rusqlite` (bundled), `serde`, `serde_json`, `regex`, `percent-encoding`, `url`, `chrono`, `futures`, `bytes`, `thiserror`, `base64`, `tokio-socks`; UI: `eframe`/`egui`, `egui_extras`, `egui-phosphor`, `rfd`, `directories` |
| Almacenamiento | SQLite: `cache.db` (desechable) y `app.db` (respaldo), modo WAL |
| Pruebas | `cargo test` (unitarias + integración con servidor FTP y HTTP locales efímeros); capturas de UI reproducibles para revisión visual |
| Plataformas | Linux x86_64 (X11 y Wayland), Windows 10/11 x86_64 |
| Tipo de proyecto | Aplicación de escritorio, workspace con biblioteca + binario |
| Metas de rendimiento | NFR-001…005 de la especificación |
| Restricciones | Principios I–IX de la constitución; solo lectura; host fijado |
| Escala | ~30.000 carpetas y ~12.500 archivos indexados; listados de hasta ~2.000 entradas |

## 3. Constitution Check

| Principio | Cumple | Cómo |
|---|---|---|
| I Privacidad | ✅ | Sin telemetría; solo catálogo y proxy; contraseñas fuera de BD y registros; `Report.proxy` enmascara credenciales (prueba `proxy_credentials_are_masked`) |
| II Núcleo primero | ✅ | Crate `novaflix-core` sin UI; la app solo consume su API |
| III Red honesta | ✅ | Módulo `transport::probe` + mensajes accionables + barra de estado con protocolo |
| IV Solo lectura | ✅ | `rclone`-style read-only: el cliente FTP solo emite `USER/PASS/TYPE/OPTS/EPSV/PASV/CWD/MLSD/LIST/SIZE/REST/RETR/QUIT` |
| V Verificar | ✅ | Pruebas de parsers y de transporte contra FTP real local (`rclone serve ftp`/servidor de prueba en proceso) |
| VI Seguridad | ✅ | TLS verificado; allowlist de host; saneado de nombres; `.part`; `cargo audit`/CodeQL |
| VII Rendimiento/accesibilidad | ✅ | Hilos separados; medición en CI (SC-004); tokens con contraste AA |
| VIII Distribución | ✅ | Todo desde CI por etiqueta; `SHA256SUMS`; repos firmados |
| IX Continuidad | ✅ | Migración con copia previa; nombres públicos conservados |

**Resultado**: sin violaciones. No se requiere *Complexity Tracking*.

## 4. Estructura del proyecto

### 4.1 Documentación (esta funcionalidad)

```
.specify/memory/constitution.md
specs/001-novaflix-v2/
├── spec.md              # qué y por qué
├── plan.md              # este archivo
├── research.md          # decisiones y alternativas
├── data-model.md        # esquema SQLite y migración v1→v2
├── contracts/
│   ├── core-api.md      # API pública del núcleo
│   ├── events.md        # eventos núcleo → UI
│   ├── transport.md     # contrato HTTP/FTP/diagnóstico
│   └── ui-screens.md    # pantallas, estados y flujos
├── design-system.md     # tokens, tipografía, movimiento, componentes
├── branding.md          # icono, logotipo y activos
├── release-and-ci.md    # estrategia de repo, ramas, versiones, CI/CD, canales
├── quickstart.md        # validación de extremo a extremo
└── tasks.md             # tareas ordenadas con dependencias y estado
docs/adr/                # registros de decisiones de arquitectura
```

### 4.2 Código fuente

```
Cargo.toml                       # workspace
crates/
├── novaflix-core/
│   ├── Cargo.toml
│   ├── src/
│   │   ├── lib.rs               # Core, Event, reexportaciones
│   │   ├── error.rs             # Error tipado + helper lock()
│   │   ├── norm.rs              # normalización leet, FTS, año            (de v1)
│   │   ├── catalog.rs           # constantes del sitio, parsers Apache     (de v1)
│   │   ├── settings.rs          # ajustes tipados sobre la tabla settings
│   │   ├── db.rs                # esquema, migraciones, helpers            (de v1)
│   │   ├── index.rs             # bootstrap, búsqueda, browse, crawl       (de v1)
│   │   ├── downloads.rs         # cola, reanudación, carpeta recursiva     (de v1)
│   │   ├── library.rs           # favoritos, listas, contadores, M3U, play (de v1)
│   │   ├── migrate.rs           # v1 → v2
│   │   └── transport/
│   │       ├── mod.rs           # Mode, ProxyChoice, Net, Download
│   │       ├── http.rs          # listados y descargas por HTTP(S)
│   │       ├── ftp.rs           # cliente FTP propio
│   │       └── probe.rs         # diagnóstico de red
│   └── tests/                   # integración: FTP real, HTTP simulado, migración
└── novaflix-app/
    ├── Cargo.toml
    ├── build.rs                 # icono de Windows (recurso .ico)
    ├── assets/                  # fuentes, iconos
    └── src/
        ├── main.rs              # arranque, runtime, ventana
        ├── app.rs               # estado de la aplicación y bucle
        ├── theme.rs             # tokens y estilo egui
        ├── motion.rs            # muelles y transiciones
        ├── cover.rs             # portadas generadas
        ├── widgets/             # tarjeta, estantería, chip, anillo de progreso…
        └── screens/             # home, search, browse, detail, library, downloads, settings
packaging/                       # deb, AppImage, NSIS, WiX, AUR, winget, desktop
.github/workflows/               # ci, release, apt, pacman, aur, codeql
```

**Decisión de estructura**: workspace de dos crates (más un tercero opcional `novaflix-cli` en el futuro). Se descartó un único crate para respetar el Principio II y permitir que el núcleo se pruebe y se publique sin dependencias gráficas.

## 5. Arquitectura

### 5.1 Hilos y flujo de datos

```
┌────────────── UI (hilo principal, egui) ──────────────┐
│  pinta estado · lee eventos · llama a Core            │
└──────┬─────────────────────────────▲───────────────────┘
       │ órdenes (síncronas o spawn) │ Event (mpsc) + repaint()
┌──────▼─────────────────────────────┴───────────────────┐
│                    novaflix-core::Core                  │
│  Mutex<Connection> cache.db   Mutex<Connection> app.db  │
│  RwLock<Arc<Net>>             Semaphore descargas       │
└──────┬─────────────────────────────────────────────────┘
       │ tokio runtime (hilos de trabajo)
┌──────▼──────────┐   ┌─────────────────────┐
│ http.rs         │   │ ftp.rs              │
│ reqwest+rustls  │   │ tokio TCP / SOCKS5  │
└──────┬──────────┘   └──────────┬──────────┘
       └──── proxy (sistema/directo/personalizado) ────► visuales.uclv.cu
```

### 5.2 Selección de transporte

`Net::active_mode()`:

| `mode` configurado | Diagnóstico FTP | Transporte usado | Ante fallo FTP |
|---|---|---|---|
| Auto | ✓ alcanzable | FTP | Cae a HTTP en esa operación |
| Auto | ✕ | HTTP | — |
| HTTP | (ignorado) | HTTP | — |
| FTP | (ignorado) | FTP | Error explicado + botón «Cambiar a HTTP» |

### 5.3 Rutas canónicas

Todas las rutas internas son **canónicas**: absolutas, con `/` inicial, codificadas como en las URL de Apache (`%20`, `%28`…), y con `/` final para carpetas. Cada transporte traduce:

- HTTP: `BASE + ruta` (con `' '` → `%20`).
- FTP: `decode_name(ruta)` sin `/` final → comando `CWD`/`RETR`.
- Pantalla: `decode_name(nombre)`.

Esto mantiene compatibilidad binaria con `app.db` y `cache.db` de v1 (FR-005).

### 5.4 Concurrencia contra el servidor

| Recurso | Límite por defecto | Ajuste |
|---|---|---|
| Descargas activas | 1 | 1–3 |
| Listados simultáneos en rastreo | 6 | fijo |
| Reintentos de listado | 3, espera 400 ms × intento | fijo |
| Tiempo de espera HTTP | 45 s (listados), sin límite práctico en descargas | fijo |
| Tiempo de espera FTP (E/S) | 20 s | fijo |

## 6. Fases de implementación

Cada fase termina con criterios de salida verificables. El detalle está en [tasks.md](tasks.md).

| Fase | Contenido | Criterio de salida |
|---|---|---|
| **0 · Fundaciones** | Preservar v1, workspace, constitución, specs, ADRs | `git branch` muestra `v1`; `cargo check` pasa; documentos revisados |
| **1 · Núcleo de datos** | `norm`, `catalog`, `db`, `settings`, `migrate`, `index` | Pruebas de parsers y esquema en verde; fixture v1 migra (SC-005) |
| **2 · Transporte** | `http`, `ftp`, `probe`, `Net` | Pruebas contra FTP real local; diagnóstico clasifica los 5 escenarios (SC-002) |
| **3 · Descargas y biblioteca** | cola, reanudación, carpeta recursiva, favoritos, listas, M3U | Descarga reanudada idéntica byte a byte (SC-003) |
| **4 · Interfaz** | tema, tarjetas, estanterías, pantallas, ficha, descargas, ajustes | Capturas de cada pantalla; recorrido de `quickstart.md` |
| **5 · Marca y empaquetado** | icono, `.ico/.icns/.png`, `.desktop`, cargo-packager | Instaladores generados en Linux; WiX/NSIS validados en CI Windows |
| **6 · CI/CD** | ci, release, apt, pacman, aur, winget, audit | Etiqueta `v2.0.0-alpha.1` publica artefactos y `SHA256SUMS` |
| **7 · Endurecimiento** | rendimiento, accesibilidad, migración real, documentación, README | SC-001…SC-008 cumplidos; paso a beta y fusión en `main` |

## 7. Estrategia de pruebas

| Nivel | Qué cubre | Cómo |
|---|---|---|
| Unitarias | `norm`, parsers de Apache/MLSD/LIST/PASV/EPSV, `encode_name`/`decode_name`, enmascarado de proxy, esquema | `cargo test -p novaflix-core` |
| Integración · FTP | login, `CWD`, `MLSD`/`LIST`, `SIZE`, `REST`+`RETR`, reanudación, SOCKS5 | Servidor FTP efímero local sobre un directorio temporal |
| Integración · HTTP | listados, `Range`, 206/200, redirección a otro host (rechazo), 407 | Servidor HTTP local en el propio test (tokio) |
| Integración · Proxy | `CONNECT` 200/403/407 y veredicto del diagnóstico | Proxy de pruebas en proceso |
| Migración | `app.db` de v1 → v2 | *Fixture* generada con el esquema de v1 (`git show v1:...`) |
| Rendimiento | búsqueda p95, arranque | Índice sintético de 30.000 carpetas, `criterion` o medición propia |
| Interfaz | capturas por pantalla y estado; contraste de tokens | Ejecución con captura del *framebuffer* y revisión visual; cálculo automático de contraste en pruebas |
| Humo en CI | la app arranca y sale | `novaflix --self-test` (sin ventana) |

## 8. Riesgos y mitigaciones

| ID | Riesgo | Prob. | Impacto | Mitigación |
|---|---|---|---|---|
| R1 | El FTP real exige credenciales no públicas (C-2) | Alta | Medio | Campos de usuario/clave; HTTP sigue siendo el camino por defecto; mensaje claro ante 530 |
| R2 | Proxy HTTP sin CONNECT a :21 en la mayoría de redes | Alta | Bajo | Es el caso diseñado: el diagnóstico lo explica y Auto usa HTTP |
| R3 | `egui` no alcanza el pulido visual deseado (sin desenfoque real) | Media | Medio | Sistema de diseño propio con capas translúcidas, degradados y muelles; ADR-0002 deja puerta a Slint si la licencia lo permite |
| R4 | Sin pósters, la interfaz parece «vacía» | Media | Medio | Portadas generadas con identidad por título; portada destacada con degradado |
| R5 | No se puede probar Windows en el entorno de desarrollo (Linux) | Alta | Medio | CI en `windows-latest`; código específico de Windows aislado y mínimo (`cfg(windows)`) |
| R6 | `cargo-packager` no cubre algún formato (AppImage/MSI) | Media | Medio | Alternativas documentadas: `cargo-deb`, `cargo-wix`, `linuxdeploy` (research.md §7) |
| R7 | Cambio de nombres de artefactos rompe PKGBUILD, winget y pacman | Media | Alto | Congelar nombres de artefacto en `release-and-ci.md` y probarlos en un release candidato |
| R8 | Pérdida de datos al migrar | Baja | Alto | Copia `*.v1.bak` previa; migración transaccional; *fixture* de pruebas |
| R9 | El servidor limita peticiones y bloquea el rastreo profundo | Media | Bajo | Rastreo opcional, 6 simultáneas, parada inmediata, reintentos con espera |
| R10 | Licencia de dependencias (p. ej. fuentes) incompatible con PolyForm NC | Baja | Medio | Solo fuentes con licencia abierta (Inter: OFL); `cargo deny licenses` |

## 9. Seguimiento de complejidad

*Sin entradas.* Ninguna decisión viola la constitución.
