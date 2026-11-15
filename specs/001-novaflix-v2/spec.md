# Especificación de funcionalidad: NovaFlix v2

**Rama de funcionalidad**: `v2/rust-rewrite`
**Creada**: 2026-11-13
**Estado**: Borrador aprobado para planificación
**Entrada**: «Reescribir NovaFlix en Rust para Windows y Linux, con una interfaz de aplicación de streaming que mezcle los paradigmas de diseño de Meta y Apple, que use el protocolo normal de la web (HTTP) y también FTP a elección del usuario, y que resuelva la situación de quien está detrás de un proxy corporativo».

> Esta especificación describe **qué** y **por qué**. El **cómo** está en `plan.md`, `research.md`, `data-model.md` y `contracts/`. Las decisiones de diseño visual están en `design-system.md`.

---

## 1. Resumen

NovaFlix v2 es una aplicación de escritorio nativa (sin navegador embebido) para explorar y descargar el catálogo de `visuales.uclv.cu`. Conserva todo lo que hacía v1 (búsqueda instantánea, explorador, favoritos, listas, descargas con reanudación) y añade:

1. Una interfaz con lenguaje visual de **aplicación de streaming**: inicio con portada y estanterías, detalle por título, descargas con progreso circular.
2. **Dos protocolos de transferencia**, HTTP(S) y FTP, elegibles por el usuario, con un modo automático.
3. Un **diagnóstico de red** que explica qué funciona desde el equipo del usuario y por qué, y que resuelve el caso real que originó el proyecto: un proxy HTTP corporativo que permite HTTPS pero bloquea el puerto 21.
4. Un binario único, sin WebView2 ni WebKitGTK.

## 2. Contexto y problema

### 2.1 Qué es v1

Aplicación Tauri 2 + React 19 (`v0.1.0`, 2026-10-27). Indexa en SQLite/FTS5 el árbol publicado por el sitio (`listado.html`, ~30.000 carpetas en ~1 s) y lista carpetas en vivo leyendo el índice de Apache. Se distribuye por GitHub Releases (`.deb`, `.AppImage`, NSIS, MSI, DMG), repositorio APT y pacman propios (firmados, en GitHub Pages), AUR, winget y Homebrew.

### 2.2 Qué falla hoy

| # | Problema | Evidencia |
|---|---|---|
| P1 | v1 solo habla HTTP. Quien prefiere FTP debe usar otro cliente (FileZilla). | README v1 |
| P2 | Detrás de un proxy corporativo, FileZilla da *timeout* sin explicación. | Diagnóstico del 2026-11-13: DNS directo no resuelve; `CONNECT visuales.uclv.cu:21` al proxy Squid responde `403 ERR_ACCESS_DENIED`; HTTPS por el proxy responde 200. |
| P3 | La interfaz de v1 es un buscador de «estilo plano corporativo»; no comunica un catálogo de películas y series. | `docs/screenshots/search.png` |
| P4 | v1 depende de WebView2 (Windows) y WebKitGTK 4.1 (Linux), con dependencias de sistema pesadas. | `CONTRIBUTING.md`, `PKGBUILD` |
| P5 | Los artefactos de Arch (`.pkg.tar.zst`, `.tar.gz` del AUR) se construyeron a mano; CI solo produce deb/AppImage/NSIS/MSI/DMG. | `release.yml`, `NovaFlix-release/` |

### 2.3 Hechos verificados sobre el servidor

Comprobados el 2026-11-13 desde una red con proxy; son la base de varias decisiones.

- `https://visuales.uclv.cu/` responde `200`, `Server: Apache/2.4.57 (Debian)`, con índice de directorios (`mod_autoindex`).
- La raíz contiene, entre otras, las carpetas `Documentales`, `Festivales y Premios de cine`, `Infantiles`, `LaMochila`, `Libros`, `MP3s`, `PaQueTeDuqes`, `Peliculas`, `Peticiones`, `ProgramacionCompetitiva`, `Reality Shows`, `Recientes`, `Series` y los ficheros `REDES_SOCIALES.txt`, `STREAMING.txt`.
- Una petición `ftp://visuales.uclv.cu/` por la pasarela FTP del proxy responde `401`: **el servidor FTP exige credenciales o no admite acceso anónimo por esa vía**. Este dato está sin resolver (ver §10, C-2).

## 3. Alcance

### 3.1 Dentro de alcance

- Linux x86_64 y Windows x86_64.
- Catálogo único: `visuales.uclv.cu` (host fijo y permitido en el núcleo).
- Búsqueda, exploración, favoritos, listas, «para ver», descargas, exportación M3U, reproducción en reproductor externo.
- Transportes HTTP(S) y FTP; diagnóstico de red; proxy del sistema, directo o personalizado (HTTP/HTTPS/SOCKS5).
- Migración automática de datos de v1.
- Distribución: GitHub Releases, APT, pacman, AUR, winget.

### 3.2 Fuera de alcance (v2.0)

| Elemento | Motivo | Cuándo |
|---|---|---|
| macOS | Se pidió Windows y Linux. `egui` y `cargo-packager` lo permitirían; el cask de Homebrew de v1 queda congelado en `0.1.0`. | v2.1, si hay demanda (ADR-0004) |
| Reproductor integrado | Se mantiene el reproductor externo (VLC/mpv/celluloid). | Futuro |
| Pósters y metadatos de TMDB u otros servicios | Violaría el Principio I (no contactar otros hosts). Se generan portadas locales. | Descartado |
| Subidas, borrado o renombrado remoto | Principio IV. | Descartado |
| FTPS / SFTP | El servidor no los ofrece, hasta donde se sabe. | Futuro |
| Soporte de otros catálogos | El host está fijado por seguridad. | Futuro, con enmienda |
| Android / iOS | — | Descartado |

## 4. Historias de usuario

Prioridades: **P1** imprescindible para publicar 2.0.0 · **P2** necesaria para 2.0.0 · **P3** deseable.

### US-1 · Buscar y descargar sin configurar nada (P1)

*Como persona detrás de un proxy corporativo, quiero abrir NovaFlix, buscar un título y descargarlo sin tocar ninguna configuración de red.*

**Por qué P1**: es el caso de uso central y el que falló con FileZilla.

**Prueba independiente**: en una red cuyo único camino a internet sea un proxy HTTP con autenticación que permite 443 y bloquea 21, abrir la app, buscar «wicked» y descargar un archivo hasta completarlo.

**Escenarios de aceptación**

1. **Dado** un equipo con `HTTPS_PROXY=http://usuario:clave@host:3128`, **cuando** se abre NovaFlix por primera vez, **entonces** la aplicación usa ese proxy sin pedir configuración y carga el índice.
2. **Dado** el índice cargado, **cuando** el usuario escribe «wick3d» en el buscador, **entonces** aparece «Wicked» entre los resultados en menos de 50 ms (normalización leet de v1).
3. **Dado** un resultado de archivo, **cuando** el usuario pulsa *Descargar*, **entonces** el archivo aparece en la cola con progreso y, al terminar, existe en la carpeta de descargas con su nombre original y tamaño correcto.
4. **Dado** que el proxy exige autenticación, **cuando** las credenciales son inválidas (407), **entonces** la aplicación muestra «El proxy rechazó las credenciales» y abre Ajustes → Red.

### US-2 · Explorar el catálogo como una app de streaming (P1)

*Como persona que quiere ver qué hay, quiero una portada con estanterías y una ficha por título, no una lista de carpetas.*

**Prueba independiente**: con el índice cargado, la pantalla de inicio muestra al menos las estanterías *Recientes*, *Continuar descargando*, *Favoritos* y una por cada sección raíz del catálogo; pulsar un título abre su ficha con sus archivos.

**Escenarios**

1. **Dado** el índice cargado, **cuando** se abre *Inicio*, **entonces** se ve una portada destacada y estanterías horizontales desplazables con tarjetas de 2:3.
2. **Dado** una tarjeta de carpeta, **cuando** se pulsa, **entonces** se abre una ficha con título limpio (sin etiquetas de release), año, sección, lista de archivos con tamaño y los botones *Descargar*, *Reproducir*, *Favorito*, *Añadir a lista*.
3. **Dado** que no hay pósters en el servidor, **cuando** se pinta una tarjeta, **entonces** se genera una portada determinista a partir del nombre (misma carpeta → misma portada) con el tipo y el año.
4. **Dado** el modo «reducir movimiento» del sistema, **cuando** se navega, **entonces** no hay animaciones de desplazamiento ni transiciones con rebote.

### US-3 · Elegir el protocolo y entender la red (P1)

*Como usuario, quiero elegir entre HTTP y FTP y, si algo no funciona, que la aplicación me diga por qué y qué está usando.*

**Prueba independiente**: en Ajustes → Red ejecutar el diagnóstico en la red del §2.2 y obtener el veredicto «Tu proxy no deja pasar FTP (403). NovaFlix usará HTTP».

**Escenarios**

1. **Dado** Ajustes → Red, **cuando** se pulsa *Diagnosticar*, **entonces** se muestran, con ✓/✕ y milisegundos, los pasos DNS, FTP :21, túnel por proxy :21 (si hay proxy) y HTTPS, más un resumen en lenguaje llano.
2. **Dado** el modo *Automático*, **cuando** FTP es alcanzable, **entonces** se usa FTP; **cuando** no lo es, se usa HTTP; y un fallo puntual de FTP en una operación cae a HTTP sin que el usuario pierda el catálogo.
3. **Dado** el modo *FTP* forzado y FTP inaccesible, **cuando** se intenta listar, **entonces** se muestra el motivo exacto y un botón *Cambiar a HTTP*; no hay fallback silencioso.
4. **Dado** el modo *HTTP*, **cuando** se descarga, **entonces** se usa `Range` para reanudar.
5. **Dado** un proxy HTTP personalizado, **cuando** se elige *FTP*, **entonces** la aplicación avisa de que un proxy HTTP no puede tunelar FTP y sugiere SOCKS5 o HTTP.
6. **Dado** un proxy `socks5://`, **cuando** se elige FTP, **entonces** las conexiones de control y de datos salen por el proxy.
7. **Dado** una barra de estado, **cuando** la app está en uso, **entonces** muestra siempre el protocolo activo (HTTP o FTP) y el proxy (con la contraseña oculta).

### US-4 · Descargas robustas (P2)

*Como usuario con una conexión inestable, quiero que las descargas se reanuden y que pueda bajar carpetas enteras.*

1. **Dado** una descarga interrumpida (corte de red, cierre de la app), **cuando** se reanuda, **entonces** continúa desde el último byte (`.part` + `Range`/`REST`) y el archivo final coincide en tamaño con el remoto.
2. **Dado** una carpeta, **cuando** se pulsa *Descargar carpeta*, **entonces** se muestra el total de archivos y bytes y se pide confirmación antes de encolar, conservando la estructura de subcarpetas.
3. **Dado** la cola, **cuando** hay varias descargas, **entonces** solo una está activa (configurable 1–3) y las demás esperan en orden.
4. **Dado** un nombre remoto con `../` o caracteres de control, **cuando** se guarda, **entonces** se sanea y nunca se escribe fuera de la carpeta de descargas.
5. **Dado** que no hay espacio en disco, **cuando** falla la escritura, **entonces** la descarga pasa a *Error* con el motivo y se puede reintentar.
6. **Dado** que existe un archivo con el mismo nombre, **cuando** se descarga de nuevo, **entonces** se pregunta si reemplazar, omitir o conservar ambos.

### US-5 · Biblioteca personal (P2)

Favoritos por archivo y por carpeta (con etiqueta y nota), listas ordenables (reproducción) y lista de «para ver» con estado *visto*, exportación M3U/M3U8 y reproducción con VLC/mpv/celluloid/totem.

1. **Dado** un favorito, **cuando** se reinicia la app, **entonces** persiste.
2. **Dado** una lista, **cuando** se reordena arrastrando, **entonces** el orden se guarda.
3. **Dado** una lista, **cuando** se exporta, **entonces** se genera un `.m3u8` con URLs absolutas válidas.
4. **Dado** que no hay reproductor instalado, **cuando** se pulsa *Reproducir*, **entonces** la app lo explica y ofrece copiar la URL.

### US-6 · Migración desde v1 (P2)

1. **Dado** un directorio de datos de v1 (`~/.local/share/com.novatic.novaflix/` o `%APPDATA%\com.novatic.novaflix\`) con `app.db`, **cuando** se abre v2 por primera vez, **entonces** se hace una copia `app.db.v1.bak`, se migra el esquema y se conservan favoritos, listas, contadores, ajustes e historial de descargas.
2. **Dado** `cache.db` de v1, **cuando** v2 arranca, **entonces** se reutiliza si el esquema es compatible o se reconstruye sin intervención.
3. **Dado** una migración fallida, **cuando** ocurre, **entonces** la app arranca con datos vacíos, conserva intacta la copia de v1 y muestra un aviso con su ruta.

### US-7 · Índice profundo y modo sin conexión parcial (P3)

Rastreo opcional (BFS, 6 listados simultáneos) para indexar también archivos sueltos; el catálogo ya visitado se puede navegar sin red durante el TTL de la caché.

### US-8 · Instalación y actualización por los canales existentes (P2)

1. **Dado** una instalación de v1 desde el repositorio APT/pacman, **cuando** se publica v2, **entonces** `apt upgrade nova-flix` / `pacman -Syu` la actualiza.
2. **Dado** Windows, **cuando** se ejecuta `winget upgrade Novatic.NovaFlix`, **entonces** se instala v2 sobre v1.
3. **Dado** cualquier canal, **cuando** se descarga un artefacto de GitHub Releases, **entonces** su SHA-256 coincide con `SHA256SUMS`.

## 5. Requisitos funcionales

### Catálogo e índice
- **FR-001** El sistema MUST cargar el árbol publicado (`/listado.html`) en un índice local y dejarlo utilizable en ≤ 3 s sobre una conexión normal (v1: ~1 s).
- **FR-002** El sistema MUST listar cualquier carpeta en vivo, con caché de TTL configurable (por defecto 12 h) y límite de tamaño con expulsión LRU (por defecto 64 MB).
- **FR-003** El sistema MUST buscar por texto con prefijos y normalización (acentos, separadores, *leet*, etiquetas de release) y filtrar por sección, año, tipo (carpeta/archivo) y extensión.
- **FR-004** El sistema SHOULD ofrecer un rastreo profundo opcional con progreso y botón de parada.
- **FR-005** Las rutas MUST almacenarse en forma canónica (codificadas como las URL de Apache) para mantener compatibilidad con v1.

### Transporte
- **FR-010** El sistema MUST ofrecer los modos *Automático*, *HTTP* y *FTP*, persistidos en ajustes.
- **FR-011** El transporte HTTP MUST usar el proxy del sistema por defecto (variables de entorno en Linux; proxy del sistema en Windows), con opciones *Directo* y *Personalizado*.
- **FR-012** El transporte FTP MUST implementar: login (anónimo o con usuario), `TYPE I`, `EPSV`/`PASV`, `MLSD` con alternativa `LIST`, `SIZE`, `REST` + `RETR`.
- **FR-013** El transporte FTP MUST poder salir por un proxy SOCKS5 (control y datos).
- **FR-014** El sistema MUST ejecutar un diagnóstico (DNS, FTP, CONNECT :21 por proxy, HTTPS) y MUST traducirlo a un resumen y una recomendación.
- **FR-015** En modo *Automático*, el sistema MUST preferir FTP solo si el diagnóstico lo confirma, y MUST caer a HTTP ante un fallo puntual.
- **FR-016** En modo *FTP* o *HTTP* forzado, el sistema MUST NOT cambiar de protocolo sin acción del usuario.
- **FR-017** El sistema MUST rechazar redirecciones a hosts distintos del catálogo.

### Descargas
- **FR-020** Cola persistente con estados `queued`, `active`, `paused`, `done`, `error`, `cancelled`.
- **FR-021** Reanudación por `Range` (HTTP) o `REST` (FTP) sobre un archivo `.part`; si el servidor no admite reanudar, reiniciar desde cero con aviso.
- **FR-022** Progreso en vivo (bytes, porcentaje, velocidad, tiempo restante) con actualización ≤ 200 ms.
- **FR-023** Descarga recursiva de carpetas con confirmación previa y estructura conservada.
- **FR-024** Límite de velocidad opcional.
- **FR-025** Verificación de tamaño al terminar (si el servidor lo informó).
- **FR-026** Contadores por ruta (intentos y completadas), heredados de v1.

### Biblioteca
- **FR-030** Favoritos de archivo y de carpeta con etiqueta y nota.
- **FR-031** Listas de tipo *playlist* (ordenable) y *watchlist* (con estado visto).
- **FR-032** Exportación M3U/M3U8 y reproducción con reproductor externo configurable.
- **FR-033** Historial de las últimas búsquedas.

### Interfaz
- **FR-040** Pantallas: Inicio, Buscar, Explorar, Ficha de título, Biblioteca (Favoritos y Listas), Descargas, Ajustes.
- **FR-041** Portadas generadas de forma determinista (sin red) para carpetas y archivos.
- **FR-042** Tema oscuro por defecto, tema claro y seguimiento del sistema.
- **FR-043** Atajos: `Ctrl+K` buscar, `Ctrl+L` ir a ruta, `Esc` cerrar ficha, `←/→/↑/↓` moverse por rejillas, `Enter` abrir.
- **FR-044** Barra de estado con protocolo activo, proxy y estado del índice.
- **FR-045** Mensajes de error accionables (Principio III).

### Datos y migración
- **FR-050** Dos bases SQLite: `cache.db` (desechable) y `app.db` (respaldo), en el mismo directorio que v1.
- **FR-051** Migración de esquema versionada (`PRAGMA user_version`) y copia de seguridad previa.

### Distribución
- **FR-060** CI MUST construir y publicar: `.deb`, `.AppImage`, `.tar.gz` (binario + `.desktop` + iconos), `.pkg.tar.zst` (Arch), instalador NSIS y MSI, y `SHA256SUMS`.
- **FR-061** CI MUST publicar repositorios APT y pacman firmados y actualizar los manifiestos de AUR y winget.

## 6. Requisitos no funcionales

| ID | Requisito | Medida |
|---|---|---|
| NFR-001 | Arranque en frío a pantalla útil | < 1,5 s con caché caliente |
| NFR-002 | Latencia de búsqueda | < 50 ms (p95) sobre 30.000 carpetas |
| NFR-003 | Fluidez de UI | ≥ 60 fps al desplazar estanterías (sin hilos bloqueados) |
| NFR-004 | Memoria en reposo | < 200 MB con índice cargado |
| NFR-005 | Tamaño del binario | < 25 MB (Linux, `strip`, LTO) |
| NFR-006 | Dependencias de sistema (Linux) | Solo librerías gráficas estándar (X11/Wayland, GL/Vulkan); sin WebKitGTK |
| NFR-007 | Dependencias de sistema (Windows) | Ninguna (sin WebView2) |
| NFR-008 | Accesibilidad | Contraste WCAG AA, navegación completa por teclado, zoom 80–200 %, reducir movimiento |
| NFR-009 | Seguridad | Principio VI; sin `unsafe` propio; `cargo audit` limpio |
| NFR-010 | Privacidad | Principio I; ninguna conexión a hosts ajenos al catálogo y al proxy |
| NFR-011 | Internacionalización | Español (es) como idioma base; textos aislados para permitir inglés después |
| NFR-012 | Robustez | Ningún `unwrap`/`expect` en rutas de usuario; errores tipados |

## 7. Entidades clave

Detalle y esquema SQL en `data-model.md`.

- **Entrada de catálogo** (carpeta o archivo): ruta canónica, nombre, nombre normalizado, sección, año, extensión, tamaño, fecha.
- **Listado cacheado**: ruta, carga útil serializada, bytes, instante de lectura.
- **Favorito**: tipo (archivo/carpeta), ruta, etiqueta, nota.
- **Lista**: nombre, tipo (*playlist* | *watchlist*), elementos con posición y estado visto.
- **Descarga**: ruta remota, destino, bytes totales/hechos, estado, error, protocolo usado.
- **Contador**: intentos y completadas por ruta.
- **Ajuste**: clave/valor.
- **Informe de red**: resultado del diagnóstico (no se persiste con secretos).

## 8. Casos límite

| Caso | Comportamiento esperado |
|---|---|
| El servidor sirve nombres en Latin-1 y UTF-8 mezclados | Decodificar UTF-8 y, si falla, Latin-1 (como v1). |
| Nombre con `%`, espacios, paréntesis, `ñ`, `#`, `&` | Round-trip exacto entre ruta canónica, URL y ruta FTP. Los enlaces escapan `&` como `&amp;`: se desescapa al leer. |
| El servidor cambia el formato de sus índices (tabla ISO ↔ `<pre>`) | El analizador reconoce ambos; una prueba con HTML real captura regresiones. |
| `listado.html` inaccesible pero FTP sí | Rastreo superficial por FTP (profundidad 3) para construir el índice. |
| `Range` ignorado por el servidor (200 en lugar de 206) | Reiniciar el `.part` y avisar. |
| El proxy responde 407 | Mensaje específico, enlace a Ajustes → Red. |
| El proxy responde 403 a `CONNECT :21` | Veredicto «tu proxy no deja pasar FTP»; modo Auto usa HTTP. |
| Servidor FTP detrás de NAT que anuncia IP privada en `PASV` | Usar el host de control. |
| `MLSD` no soportado (500/502) | Alternativa a `LIST` con análisis estilo Unix. |
| Cierre de la app durante una descarga | La cola se restaura en `paused`; el `.part` se conserva. |
| Reloj del sistema erróneo | No afecta a la validez de la caché más allá de re-lecturas. |
| Disco lleno | Error accionable; no se corrompe el `.part`. |
| Dos instancias a la vez | La segunda enfoca la primera (bloqueo de instancia única). |
| Pantalla pequeña (< 960×640) | Mínimo de ventana impuesto; la rejilla reduce columnas. |
| Sin reproductor externo | Ofrecer copiar la URL. |
| Resultado de búsqueda vacío | Estado vacío con sugerencias (últimas búsquedas, secciones). |

## 9. Criterios de éxito medibles

- **SC-001** Una persona detrás del proxy descrito en §2.2 completa «buscar → descargar» en < 60 s desde la primera apertura, sin tocar ajustes.
- **SC-002** El diagnóstico identifica la causa correcta en el 100 % de los escenarios de prueba (directo, proxy que bloquea :21, proxy con 407, sin red, SOCKS5).
- **SC-003** Una descarga de ≥ 1 GB interrumpida tres veces se completa con tamaño idéntico al remoto.
- **SC-004** Búsqueda p95 < 50 ms y arranque < 1,5 s (NFR-001/002) medidos en CI sobre un índice sintético de 30.000 carpetas.
- **SC-005** La migración de un `app.db` de v1 conserva el 100 % de favoritos, listas, contadores y ajustes (prueba con *fixture*).
- **SC-006** `apt upgrade`, `pacman -Syu` y `winget upgrade` actualizan v1 → v2 sin reinstalar (verificado manualmente en el release candidato).
- **SC-007** CI en verde (fmt, clippy, tests, audit) en Linux y Windows para cada commit de `main`.
- **SC-008** Revisión de interfaz: 0 incumplimientos de contraste AA en los tokens de `design-system.md`.

## 10. Aclaraciones

### Resueltas

| ID | Pregunta | Decisión |
|---|---|---|
| C-1 | ¿Reescribir en un repositorio nuevo o en el actual? | En el actual: rama `v1` preserva la v1; el trabajo va en `v2/rust-rewrite` y llega a `main` en la beta. Mantiene estrellas, issues, URL de instalación y firma (ADR-0001). |
| C-3 | ¿Toolkit gráfico? | `egui`/`eframe` (ADR-0002). |
| C-4 | ¿FTP con crate existente o propio? | Cliente propio sobre `tokio` (ADR-0003). |
| C-5 | ¿Pósters? | Portadas generadas localmente; no se contacta a servicios de metadatos (Principio I). |
| C-6 | ¿Versión? | `2.0.0-alpha.N` → `beta.N` → `2.0.0`. v1 era `0.1.0`; SemVer garantiza que v2 sea mayor. |

### Abiertas (requieren decisión del propietario antes de 2.0.0)

| ID | Pregunta | Efecto |
|---|---|---|
| C-2 | ¿El FTP de `visuales.uclv.cu` permite acceso anónimo? ¿Hay usuario/clave por defecto para la comunidad? | Define el usuario por defecto del modo FTP. Hoy: `anonymous`; si falla con 530, la UI pide credenciales. |
| C-7 | ¿Se mantiene macOS en 2.x? | Si sí: añadir job y cask en v2.1. |
| C-8 | ¿Firma de código en Windows (Authenticode)? | Sin ella, SmartScreen avisará. Requiere certificado. |
| C-9 | ¿Nombre del idioma de la interfaz en inglés? | NFR-011 deja la puerta abierta. |
