# Contrato: transporte HTTP / FTP y diagnóstico

**Spec**: [../spec.md](../spec.md) · **API**: [core-api.md](core-api.md) · **Investigación**: [../research.md](../research.md) §3–4

## 1. Interfaz común

```rust
impl Net {
    pub fn build(cfg: NetConfig) -> Result<Net>;
    pub fn active_mode(&self) -> Mode;                                   // Http | Ftp efectivo
    pub fn set_ftp_available(&self, ok: bool);                           // lo fija el diagnóstico
    pub async fn list_dir(&self, path: &str) -> Result<Vec<RawEntry>>;
    pub async fn open(&self, path: &str, offset: u64) -> Result<Download>;
    pub async fn fetch_listado(&self) -> Result<String>;                 // siempre HTTP
}
pub struct Download { pub total: Option<u64>, pub resumed: bool, pub stream: BoxStream<'static, Result<Bytes>> }
pub struct RawEntry { pub href: String, pub name: String, pub is_dir: bool, pub size: Option<i64>, pub mtime: Option<String> }
```

`RawEntry.href` es **relativo y codificado** (`Wicked%20%282024%29/` para carpetas, con `/` final). `name` está decodificado. `mtime` se normaliza a `YYYY-MM-DD HH:MM` en ambos transportes (el servidor actual ya lo sirve así; el formato antiguo `29-Sep-2026 05:30` se convierte).

## 2. HTTP(S)

| Operación | Petición | Notas |
|---|---|---|
| Listar | `GET https://visuales.uclv.cu/<ruta>/` | `parse_listing` reconoce la **tabla de Apache** actual (fechas ISO) y el formato `<pre>` de v1; desescapa entidades HTML; 3 intentos, espera 400 ms × intento |
| Descargar | `GET <url>` con `Range: bytes=<offset>-` si `offset > 0` | `206` → `resumed=true`, `total = content_length + offset`; `200` con `offset > 0` → `resumed=false` (el llamador reinicia el `.part`) |
| Listado completo | `GET /listado.html` | Solo HTTP |

**Cliente**: `User-Agent: NovaFlix/<versión>`; `connect_timeout` 20 s; `timeout` 45 s (listados) y 24 h (descargas); `redirect::Policy` que **solo** sigue redirecciones al mismo host (CT-08); `rustls` con raíces del sistema; `gzip` activo.

**Proxy** (`ProxyChoice`):

| Opción | Comportamiento |
|---|---|
| `System` | `reqwest` con `system-proxy`: variables `HTTP(S)_PROXY`, `ALL_PROXY`, `NO_PROXY` (Linux) y configuración del sistema (Windows) |
| `Direct` | `no_proxy()` |
| `Custom(url)` | `Proxy::all(url)`; admite `http://`, `https://`, `socks5://` y credenciales en la URL (que provienen del llavero, no de `settings`) |

**Errores HTTP a `ErrorKind`**: 407 → `ProxyAuth`; 404 → `NotFound`; fallo DNS/conexión → `Offline`; resto → `Other` con `HTTP <código>`.

## 3. FTP

### 3.1 Comandos permitidos (lista cerrada — Principio IV)

`USER`, `PASS`, `TYPE I`, `OPTS UTF8 ON`, `EPSV`, `PASV`, `CWD`, `MLSD`, `LIST`, `SIZE`, `REST`, `RETR`, `QUIT`.

Cualquier otro comando **no existe** en el código. Una prueba estática (`grep` sobre `ftp.rs` en CI) falla si aparecen `STOR`, `APPE`, `DELE`, `RMD`, `MKD`, `RNFR`, `RNTO`, `SITE` o `STOU`.

### 3.2 Sesión

1. Conectar al `host:21` (directo o por SOCKS5), tiempo máximo 20 s.
2. Esperar `220`.
3. `USER <ftp_user>`; si `331`/`332`, `PASS <clave>`; si `230`, listo; si `530`, `ErrorKind::FtpLogin`.
4. `TYPE I`; `OPTS UTF8 ON` (el fallo se ignora).
5. Operación.
6. `QUIT` (mejor esfuerzo).

Una sesión por operación (listar o descargar). No hay *pool* en 2.0: simplicidad y robustez ante servidores que cierran conexiones ociosas.

### 3.3 Conexión de datos

1. `EPSV` → `229 … (|||puerto|)`; se conecta a `host_de_control:puerto`.
2. Si `EPSV` no da `229`: `PASV` → `227 … (h1,h2,h3,h4,p1,p2)`. Se usa la IP anunciada **solo si es pública y distinta**; si es privada, de *loopback* o `0.0.0.0` se usa el host de control (servidores tras NAT).
3. Modo activo (`PORT`) **no soportado**.

### 3.4 Listado

`CWD <ruta>` → `MLSD`; si no responde `125/150` → `LIST`.

| Formato | Análisis |
|---|---|
| MLSD (RFC 3659) | Hechos `type=dir|file`, `size`, `modify=YYYYMMDDHHMMSS`; se ignoran `cdir`, `pdir` y otros |
| LIST Unix | `^([-d])[rwxsStTl-]{9}\s+\d+\s+\S+\s+\S+\s+(\d+)\s+(Mon)\s+(d)\s+(YYYY|HH:MM)\s+(nombre)$`; los enlaces simbólicos se ignoran |

Las fechas LIST sin año asumen el año actual.

### 3.5 Descarga

`SIZE` → `REST <offset>` (si `offset > 0`; `350` ⇒ `resumed`) → datos por `EPSV/PASV` → `RETR`. La `Download.stream` lee la conexión de datos en bloques de 64 KiB con límite de inactividad de 20 s y **posee** la sesión de control hasta terminar.

**Servidores que ignoran `REST`** *(hallazgo verificado el 2026-11-13 contra `rclone serve ftp`)*: un servidor puede responder `350` y aun así enviar el fichero desde el byte 0. Un cliente que confíe en el `350` corrompería el `.part`. Por eso, cuando `SIZE` es conocido, el flujo devuelve `Error::Ftp("el servidor ignoró REST…")` si recibe más de `total − offset` bytes; el gestor de descargas reacciona truncando el `.part` y reiniciando desde cero (una vez). La verificación de tamaño final (FR-025) es obligatoria en cualquier caso.

### 3.6 Proxy SOCKS5

Si `Custom(socks5://host:puerto)`: tanto la conexión de control como cada conexión de datos se abren con `Socks5Stream::connect(proxy, (host, puerto))`. Con un proxy HTTP personalizado el FTP **no** puede tunelarse y se informa (US-3.5).

### 3.7 Errores FTP a `ErrorKind`

`530` → `FtpLogin`; `550` → `NotFound`; tiempo agotado o `connection refused` → `Offline`; `421`/`425`/`426` → `Other` con el texto del servidor.

## 4. Selección y caída (Auto)

Ver [plan.md §5.2](../plan.md). En Auto, `Net::list_dir` y `Net::open` intentan FTP y, ante **cualquier** error, repiten la misma operación por HTTP. En modo `Ftp` forzado el error se devuelve intacto.

## 5. Diagnóstico (`probe::run`)

### 5.1 Pasos

| # | Paso | Cómo | Éxito |
|---|---|---|---|
| 1 | DNS | `lookup_host(host, 21)` con 6 s | Devuelve una IP |
| 2 | FTP | TCP a la IP:21 y lectura del saludo; con SOCKS5, sesión completa | El saludo empieza por `220` |
| 3 | Túnel por proxy | Solo si hay proxy `http(s)` y no hay SOCKS5: `CONNECT host:21` con `Proxy-Authorization: Basic` si hay credenciales | Respuesta `200` |
| 4 | HTTPS | `HEAD https://visuales.uclv.cu/` con el cliente configurado | `2xx` |

### 5.2 Veredicto

| FTP | HTTPS | Veredicto | `recommendation` |
|---|---|---|---|
| ✓ | ✓ | «FTP y HTTP funcionan desde aquí.» | Ftp |
| ✓ | ✕ | «FTP funciona, pero HTTP no responde.» | Ftp |
| ✕ | ✓ | Con paso 3 fallido: «Tu proxy no deja pasar FTP (<detalle>). NovaFlix usará HTTP, que sí funciona.» Sin paso 3: «FTP no es accesible desde esta red. NovaFlix usará HTTP, que sí funciona.» | Http |
| ✕ | ✕ | «No hay conexión con el servidor. Revisa la red o la configuración del proxy.» | Http |

### 5.3 Tabla de escenarios de aceptación (SC-002)

| Escenario | DNS | FTP | Túnel :21 | HTTPS | Veredicto |
|---|---|---|---|---|---|
| A. Red directa a la intranet | ✓ | ✓ | — | ✓ | FTP y HTTP funcionan |
| B. Proxy Squid que bloquea :21 *(caso real)* | ✕ | ✕ | ✕ (403) | ✓ | Proxy no deja pasar FTP; HTTP |
| C. Proxy con credenciales inválidas | ✕ | ✕ | ✕ (407) | ✕ (407) | Sin conexión; revisa el proxy |
| D. Sin red | ✕ | ✕ | — | ✕ | Sin conexión |
| E. SOCKS5 que permite :21 | — | ✓ | — | ✓ | FTP y HTTP funcionan |

### 5.4 Privacidad del informe

`Report.proxy` se muestra como `esquema://usuario:***@host:puerto`. Ningún `detail` incluye la URL completa del proxy ni la contraseña. La acción «Copiar diagnóstico» exporta el `Report` ya enmascarado.
