# Investigación y decisiones: NovaFlix v2

**Fecha**: 2026-11-13 · **Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Cada sección sigue el formato **Decisión → Razón → Alternativas descartadas**. Las decisiones con impacto estructural tienen además un ADR en `docs/adr/`.

---

## 1. Estrategia de repositorio (ADR-0001)

**Decisión**: mantener el repositorio `novatic-cu/NovaFlix`. La v1 se preserva en la rama `v1` (creada desde `main` en `899cc24`) y en la etiqueta `v0.1.0`. La v2 se desarrolla en `v2/rust-rewrite`, se fusiona en `main` al llegar a beta, y `v1` queda como rama de mantenimiento (`v1.x`) solo para correcciones críticas.

**Razón**
- Todos los canales de distribución de v1 (APT `nova-flix`, pacman `novaflix-bin`, AUR, winget `Novatic.NovaFlix`, GitHub Pages) están ligados al nombre del repositorio y a la clave de firma `F005 E863 F53A 70D6 0447 9A36 9FE8 FBEF 7C21 10F0`. Un repositorio nuevo obligaría a reinstalar y duplicaría paquetes (viola el Principio IX).
- Se conservan estrellas, issues, enlaces del README y la página `novatic-cu.github.io/NovaFlix`.
- Una rama de larga vida es suficiente: los artefactos de v1 ya están publicados y firmados; no se necesita compilar v1 en CI.

**Alternativas descartadas**
- *Repositorio nuevo `NovaFlix-v2`*: rompe actualizaciones y fragmenta la comunidad.
- *Monorepo con `v1/` y `v2/` en `main`*: ensucia el historial y confunde a CI (dos cadenas de herramientas).
- *Sustituir `main` de inmediato*: dejaría `main` sin una v1 funcional mientras v2 no está lista y dispararía los flujos de publicación (`release: published`) con un binario incompleto.

**Consecuencias**: los flujos `apt.yml`, `pacman.yml` y `aur.yml` se disparan al *publicar* un release; las prereleases (`alpha`, `beta`) **no** deben dispararlos (ver `release-and-ci.md` §6).

---

## 2. Toolkit de interfaz (ADR-0002)

**Decisión**: `egui` + `eframe` (backend `glow`, con `wgpu` como opción futura), con un sistema de diseño propio.

**Razón**
- Rust puro, un solo binario, sin webview: satisface NFR-005/006/007 y elimina WebView2 y WebKitGTK (P4).
- Licencia MIT/Apache-2.0, compatible con PolyForm Noncommercial sin condiciones adicionales.
- Control total del dibujado (degradados, sombras, formas redondeadas, mallas) necesario para tarjetas, portadas generadas, anillos de progreso y capas translúcidas.
- Ciclo de desarrollo rápido y fácil de verificar en este entorno (Linux/Wayland) con capturas reproducibles.

**Alternativas evaluadas**

| Opción | A favor | En contra |
|---|---|---|
| **Slint** | Declarativo, animaciones de primera clase, muy pulido | Licencia: GPL-3.0, o «Royalty-Free Desktop» con atribución y registro, o comercial. La GPL-3.0 es incompatible con distribuir bajo PolyForm NC; la licencia gratuita añade obligaciones y dependencia de un tercero. |
| **Iced** | Elm-like, MIT, wgpu | Animación y estilo más costosos; ecosistema de widgets menor; texto y tablas más verbosos |
| **GPUI** (Zed) | Rendimiento excelente | API inestable, soporte Windows/Linux reciente, poca documentación fuera de Zed |
| **Dioxus (webview)** / **Tauri (v1)** | Reutiliza HTML/CSS | Mantiene la dependencia de WebView2/WebKitGTK, que es justo lo que se quiere eliminar |
| **GTK4 / Qt** | Nativos | Distribución pesada en Windows, estilos difíciles de salirse del tema del sistema, enlaces no Rust puro |

**Limitaciones aceptadas**: `egui` no ofrece desenfoque real del fondo (*backdrop blur*). El efecto «cristal» se logra con capas translúcidas, degradados y bordes de 1 px (design-system.md §4). Si más adelante hace falta desenfoque real, se evaluará `wgpu` con un *shader* propio.

**Condición de revisión**: si Slint cambiara a una licencia compatible con PolyForm NC sin atribución obligatoria, reabrir esta decisión.

---

## 3. Cliente FTP propio vs. crate (ADR-0003)

**Decisión**: implementar un cliente FTP mínimo y de solo lectura sobre `tokio` (`transport/ftp.rs`, ≈ 350 líneas).

**Razón**
- Se necesita únicamente: login, `TYPE I`, `EPSV`/`PASV`, `CWD`, `MLSD`/`LIST`, `SIZE`, `REST`+`RETR`. Es un subconjunto pequeño y estable del protocolo (RFC 959, 2428, 3659).
- Control sobre el **proxy SOCKS5** tanto en la conexión de control como en la de datos, y sobre el tratamiento de **IP privadas anunciadas en `PASV`** (NAT).
- Se evita adaptar el *runtime* de la dependencia a `tokio`. Al intentar añadir `suppaftp` en el entorno, la característica que se esperaba (`tokio-rustls-aws-lc-rs`) no existía; la versión resuelta ofrecía `async`, `async-rustls`, `async-native-tls` y similares. **No se verificó** sobre qué *runtime* operan; si una revisión futura demuestra que encajan limpiamente con `tokio` y admiten SOCKS5, esta decisión puede reabrirse.
- Superficie de ataque pequeña y auditable: el Principio IV exige que el cliente **no pueda** emitir comandos de escritura; con un cliente propio esa garantía es estructural.

**Alternativas descartadas**
- *`suppaftp`*: completo, pero sin SOCKS5 y con incompatibilidades de *runtime*; incorporaría funciones de escritura que no queremos exponer.
- *`libcurl` (crate `curl`)*: soporta FTP y proxys, pero añade una dependencia C pesada y complica la compilación cruzada a Windows.
- *Reutilizar `rclone` como proceso hijo*: exige instalar un binario externo (viola NFR-006/007).

**Riesgos**: servidores FTP con comportamientos exóticos (listados no estándar, `PASV` con IP pública distinta). Mitigación: pruebas contra servidor real (rclone y otro), análisis de `LIST` con *regex* de formato Unix, y caída a HTTP en modo Auto.

**Fuera de alcance deliberado**: FTPS/SFTP, modo activo (`PORT`), subidas.

---

## 4. FTP detrás de un proxy HTTP: lo que es y no es posible

**Hechos medidos (2026-11-13)**
- `CONNECT visuales.uclv.cu:21` por el proxy Squid → `403 Forbidden`, `X-Squid-Error: ERR_ACCESS_DENIED`.
- `GET ftp://visuales.uclv.cu/` por el mismo proxy (modo pasarela FTP de Squid) → `401`: el proxy contacta con el servidor FTP, que pide autenticación.
- `GET https://visuales.uclv.cu/` por el proxy → `200`.

**Conclusiones**
1. Un cliente FTP «de verdad» (control + datos en conexiones TCP propias) **no puede** funcionar a través de este proxy: solo se pueden tunelar puertos permitidos por la ACL `SSL_ports`/`Safe_ports` de Squid.
2. La pasarela FTP de Squid existe, pero es un modo HTTP→FTP de solo lectura, con autenticación HTTP (`Authorization: Basic`), no un FTP nativo; no se puede usar desde un cliente FTP estándar como FileZilla.
3. Por tanto, el modo FTP es útil **fuera** del proxy (o con SOCKS5), y el modo HTTP es el camino universal.

**Decisión**: el diagnóstico detecta el caso y lo explica; el modo Auto elige HTTP; **no** se implementa la pasarela FTP de Squid en 2.0 (se registra como idea futura en `tasks.md`, T-F01) porque dependería de una configuración específica del proxy y de credenciales FTP aún desconocidas (C-2).

**Puente local (solución provisional documentada)**: mientras tanto, existe el truco con `rclone serve ftp` sobre el backend HTTP, que expone el sitio como FTP en `127.0.0.1`. NovaFlix v2 lo hace innecesario porque ya habla HTTP de forma nativa.

---

## 5. Persistencia

**Decisión**: SQLite con `rusqlite` (`bundled`), FTS5, WAL; dos bases (`cache.db` desechable, `app.db` de respaldo), mismos nombres, ubicaciones y esquema base que v1; versión de esquema en `PRAGMA user_version`.

**Razón**: compatibilidad directa con v1 (FR-005, FR-050/051), FTS5 ya probado con 30.000 carpetas en ~1 s, y cero servicios externos.

**Alternativas descartadas**: `sled`/`redb` (sin FTS y sin migración desde v1), JSON (lento para búsqueda), `sqlx` (async innecesario para consultas locales).

**Ubicaciones**: Linux `~/.local/share/com.novatic.novaflix/`; Windows `%APPDATA%\com.novatic.novaflix\`. Se resuelven con el crate `directories` (`ProjectDirs::from("com", "novatic", "novaflix")` produce una ruta distinta a la de v1 en Windows, por lo que se **fuerza** la ruta de v1 para conservar los datos).

---

## 6. Secretos

**Decisión**: contraseñas de proxy personalizado y de FTP en el llavero del sistema mediante el crate `keyring` (Secret Service en Linux, Credential Manager en Windows); si el llavero no está disponible, **no se guardan** y se piden en cada sesión. El proxy del sistema se lee del entorno o del registro y nunca se persiste.

**Razón**: Principio I. Guardar la contraseña en `app.db` o en un archivo de configuración la dejaría en texto plano y en copias de seguridad.

**Alternativas descartadas**: cifrado propio con clave derivada (la clave tendría que vivir junto a los datos), guardarla en claro con permisos 0600 (insuficiente en Windows).

**Estado**: el diagnóstico y el transporte ya aceptan credenciales en la URL del proxy y las enmascaran; la integración con `keyring` es la tarea T-044.

---

## 7. Empaquetado y distribución sin Tauri (ADR-0005)

**Decisión**: `cargo-packager` (CrabNebula) como empaquetador principal para `.deb`, `.AppImage`, NSIS y MSI (WiX), con tres artefactos adicionales generados por scripts: `.tar.gz` (binario + `.desktop` + iconos, usado por el AUR), `.pkg.tar.zst` (Arch, con `makepkg` en contenedor `archlinux`) y `SHA256SUMS`.

**Razón**
- Nace del empaquetador de Tauri y apunta a binarios que no son Tauri; produce los mismos formatos que v1 ya distribuía (deb, AppImage, NSIS, MSI). *Pendiente de verificar en el entorno* (tarea T-061).
- Los nombres de artefacto se fijan (release-and-ci.md §5) para no romper PKGBUILD, winget ni los flujos APT/pacman.

**Alternativas**
- `cargo-deb` + `cargo-wix` + `linuxdeploy`: más control por formato, más piezas que mantener.
- `cargo-bundle`: soporte irregular de Windows.
- *Instaladores propios (Inno Setup)*: fuera del ecosistema Rust.

**Hallazgo sobre v1 (P5)**: `release.yml` solo publica `.deb`, `.AppImage`, NSIS, MSI y DMG; el `.tar.gz` del AUR y el `.pkg.tar.zst` de pacman que referencian `PKGBUILD` y `pacman.yml` se construyeron **a mano** (carpeta `NovaFlix-release/`). v2 los automatiza.

**Riesgo**: `cargo-packager` es más joven que el empaquetador de Tauri. Mitigación: probar `deb` y `AppImage` localmente antes de cualquier etiqueta; `windows-latest` valida NSIS/MSI en CI.

---

## 8. Tipografía e iconos de interfaz

**Decisión**: **Inter** (SIL OFL 1.1) para texto, con pesos Regular (400), Medium (500) y SemiBold (600) registrados como familias con nombre en `egui`; **Phosphor Icons** (`egui-phosphor`, MIT) para iconografía.

**Razón**: Inter es la tipografía más cercana a SF Pro con licencia abierta; ya está en el sitio de v1 (`site/assets/fonts/Inter-*.woff2`). `egui` no carga `woff2`: se usan TTF. Phosphor ofrece pesos regular/fill/bold que permiten estados activos sin dibujar a mano.

**Nota de licencias**: ambas licencias permiten incrustar y redistribuir; se incluye `THIRD_PARTY_LICENSES.md` generado con `cargo about`.

---

## 9. Portadas generadas

**Decisión**: generar una portada determinista por título: semilla = hash FNV-1a del nombre normalizado; paleta de 2–3 tonos tomada de una lista curada (no HSL aleatorio) para garantizar contraste del texto; degradado diagonal + una forma geométrica suave (círculo/arco) + tipo (película, serie, música…) y año en tipografía grande.

**Razón**: sin pósters (C-5, Principio I) las tarjetas serían iguales y sosas; una portada con identidad hace que el catálogo se perciba como un catálogo de streaming. Es 100 % local, instantánea y reproducible (misma carpeta → misma portada).

**Alternativas descartadas**: TMDB/Fanart (viola el Principio I), *thumbnails* de vídeo (requieren descargar el fichero), iconos genéricos (monótono).

---

## 10. Medición de rendimiento

**Decisión**: `cargo test` con un índice sintético de 30.000 carpetas y 12.500 archivos para búsqueda (p95) y apertura de `cache.db`; el arranque a pantalla útil se mide con un temporizador interno (`--self-test --timing`).

**Razón**: SC-004 exige medir, no suponer. v1 reportaba ~1 s de arranque de índice; v2 debe igualarlo o mejorarlo.

---

## 11. Nombre del paquete y binario

**Decisión**: binario `novaflix`; paquete `.deb` y APT `nova-flix` (como v1); Arch `novaflix-bin`; winget `Novatic.NovaFlix`; identificador `com.novatic.novaflix`.

**Razón**: Principio IX. Cambiar un nombre rompe las actualizaciones.

---

## 12. Cuestiones abiertas que condicionan decisiones

| Cuestión | Qué decide | Responsable |
|---|---|---|
| C-2 acceso FTP anónimo | Usuario por defecto del modo FTP | Propietario del proyecto |
| C-7 macOS | Job extra en `release.yml` y cask | Propietario |
| C-8 firma Authenticode | Aviso de SmartScreen en Windows | Propietario |

---

## 13. MSRV (versión mínima de Rust)

**Decisión**: MSRV **1.95**. **Razón**: `eframe`/`egui` 0.36 (la versión que empareja con `egui-phosphor` 0.14, que depende de `egui` 0.36) exige Rust 1.95 (`cargo info eframe@0.36.2`, 2026-11-13). La primera estimación de 1.85 se hizo antes de elegir versiones concretas. **Consecuencia**: el job `msrv` de CI y la documentación usan 1.95; las distribuciones que empaqueten desde fuente necesitan un Rust reciente (no aplica: se distribuyen binarios).

---

## 14. Formato real del servidor (hallazgos del 2026-11-14)

Al probar contra el sitio vivo, el HTML difería de lo que asumía el analizador de v1. Es el tipo de fallo que solo aparece con datos reales (Principio V).

| Hallazgo | Evidencia | Decisión |
|---|---|---|
| Los índices de carpeta son una **tabla de Apache** (`<tr><td><a href>…</a></td><td align="right">2026-11-10 03:17 </td>…`) con **fechas ISO**, no el formato `<pre>` con `29-Sep-2026 05:30` que reconocía v1 | `curl https://visuales.uclv.cu/` | `catalog::parse_listing` reconoce **ambos**; las fechas se normalizan a `YYYY-MM-DD HH:MM` (ordenables) también en FTP |
| `listado.html` (4,7 MB, generado por `tree`) escapa `&` como `&amp;` en los enlaces. v1 guardaba la ruta con `&amp;` literal, lo que rompía la navegación a carpetas con `&` | `grep '&amp;' listado.html` (líneas 128–130) | `catalog::html_unescape` en enlaces y rutas; prueba `listado_paths_are_unescaped` |
| `listado.html` **no es UTF-8 válido** (mezcla Latin-1) | `read_to_string` falló con `InvalidData` | `catalog::decode_body`: UTF-8 y, si falla, Latin-1 (v1 ya lo hacía solo en `fetch_text`) |
| `listado.html` contiene ≈ 30.000 rutas de carpeta | prueba `real_listado_parses_cleanly` (> 20.000) | Sin cambios; el índice ligero sigue siendo viable |
| Las secciones son **categorías**, no títulos: `/Peliculas/` → `Cubanas/`, `Extranjeras/` → `<año>/` → `<título>/` | Listado de `/Peliculas/` | La portada no puede ser «una estantería por sección con sus hijos»; ver design-system §6 (Categorías + Novedades) |
| `/Recientes/` mezcla carpetas y episodios sueltos (`Abbott.Elementary.S06E06.480p…mkv`) con fecha | Listado de `/Recientes/` | Estantería **Novedades** = `/Recientes/` ordenada por fecha; tarjetas para archivos y carpetas |

**Fixtures reales** en `crates/novaflix-core/tests/fixtures/` (`apache_table_root.html`, `apache_table_recientes.html`). El servidor de pruebas HTTP imita ahora el formato de tabla. La comprobación contra el `listado.html` completo es una prueba `--ignored` (`NOVAFLIX_REAL_LISTADO`).

**Consecuencia para los datos de v1**: las rutas con `&amp;` que v1 pudiera haber guardado en `favorites`/`list_items` apuntan a carpetas inexistentes. La migración las corrige (`replace("&amp;", "&")`) — tarea T-016b.
