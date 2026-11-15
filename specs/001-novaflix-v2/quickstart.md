# Guía de validación (quickstart): NovaFlix v2

**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md)

Recorrido de extremo a extremo para comprobar que la v2 cumple la especificación. Cada paso indica el requisito que verifica. Se ejecuta en **Linux** y en **Windows** antes de cada beta y del release estable.

## 0. Requisitos

- Rust estable ≥ 1.95 (`rustup`).
- Linux: bibliotecas gráficas (X11/Wayland, GL). Windows: MSVC Build Tools.
- Acceso al catálogo por HTTPS (directo o por proxy).
- Opcional para pruebas de FTP: un servidor FTP local (p. ej. `rclone serve ftp` o el de las pruebas de integración).

## 1. Compilar y probar

```bash
git switch v2/rust-rewrite
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

**Esperado**: todo en verde. Las pruebas de integración levantan un servidor FTP y uno HTTP locales sobre directorios temporales (Principio V).

## 2. Ejecutar

```bash
cargo run --release -p novaflix
```

## 3. Escenarios

### 3.1 Primer arranque detrás de proxy → US-1 · FR-001 · FR-011

1. Exporta el proxy: `export HTTPS_PROXY=http://usuario:clave@host:3128` (o configúralo en Windows).
2. Borra datos previos: `rm -rf ~/.local/share/com.novatic.novaflix` (¡solo en un entorno de pruebas!).
3. Abre la app.

**Esperado**: aparece «Preparando el catálogo…», luego Inicio con estanterías. La barra de estado muestra `HTTP` y el proxy como `http://usuario:***@host:3128`. No se pidió ninguna configuración.

### 3.2 Búsqueda → US-1 · FR-003 · NFR-002

1. `Ctrl+K`, escribe `wick3d`.

**Esperado**: «Wicked» entre los resultados, con el contador y el tiempo («… · <50 ms»).

### 3.3 Explorar y ficha → US-2 · FR-040/041

1. Inicio → clic en una tarjeta de *Recientes*.
2. En la ficha, comprueba título limpio, año, sección y lista de archivos.
3. Cierra con `Esc`.

**Esperado**: la misma carpeta muestra siempre la misma portada generada.

### 3.4 Descarga y reanudación → US-4 · FR-020/021 · SC-003

1. En la ficha de un archivo grande, pulsa *Descargar*.
2. A mitad, **corta la red** (modo avión) o cierra la app.
3. Restablece y reabre; ve a Descargas y pulsa *Reanudar*.

**Esperado**: la descarga continúa desde el último byte (existe `<archivo>.part` entre medias); el archivo final tiene el tamaño remoto exacto (`stat`/`ls -l` contra la cabecera `Content-Length`).

### 3.5 Diagnóstico de red → US-3 · FR-014 · SC-002

1. Ajustes → Red → **Diagnosticar**.

**Esperado en la red del caso real (proxy Squid que bloquea :21)**:

| Paso | Resultado |
|---|---|
| DNS | ✕ |
| FTP :21 | ✕ |
| Túnel :21 por proxy | ✕ (`403`) |
| HTTPS | ✓ |

Veredicto: «Tu proxy no deja pasar FTP (…). NovaFlix usará HTTP, que sí funciona.»

2. Elige *FTP* → aparece el cuadro de veredicto con **Usar HTTP**.
3. *Copiar diagnóstico* y pega en un editor: **no debe aparecer la contraseña del proxy** (R-D3).

### 3.6 FTP real → US-3 · FR-012/013 · Principio V

1. Levanta un servidor FTP local: `rclone serve ftp <dir> --addr 127.0.0.1:2121 --read-only` (o usa el de pruebas).
2. En Ajustes → Red: protocolo *FTP*, host por defecto cambiado a `127.0.0.1:2121` solo en la variable de entorno de pruebas `NOVAFLIX_FTP_ADDR=127.0.0.1:2121`.
3. Explora y descarga un archivo.

**Esperado**: la barra de estado muestra `FTP`; el listado y la descarga funcionan; reanudar tras cortar usa `REST`.

### 3.7 Modo Auto con caída → FR-015

1. Modo *Automático* con FTP configurado a un puerto cerrado.
2. Explora una carpeta.

**Esperado**: el listado aparece por HTTP sin error visible; la barra muestra `HTTP`.

### 3.8 Migración desde v1 → US-6 · FR-051 · SC-005

1. Instala v1 (`v0.1.0`), crea 2 favoritos, 1 lista con 2 elementos y 1 descarga.
2. Abre v2.

**Esperado**: existe `app.db.v1.bak`; favoritos, lista y contadores están presentes; la descarga en curso aparece `paused`. Abrir v2 de nuevo no cambia nada.

### 3.9 Biblioteca y M3U → US-5

1. Marca ☆ en un título; crea una lista, añade dos elementos, reordénalos.
2. Exporta M3U.

**Esperado**: el favorito y el orden persisten tras reiniciar; el `.m3u8` abre en VLC/mpv con URLs HTTP válidas.

### 3.10 Accesibilidad → NFR-008 · SC-008

1. Navega **solo con teclado** por Inicio → Buscar → ficha → descargar.
2. Activa «reducir movimiento» (Ajustes → Apariencia).
3. Cambia la escala a 150 %.

**Esperado**: foco visible siempre; sin animaciones con movimiento; el diseño no se rompe a 150 %. La prueba automática de contraste de tokens pasa.

### 3.11 Rendimiento → NFR-001..004 · SC-004

```bash
cargo test -p novaflix-core --release perf -- --nocapture
```

**Esperado**: búsqueda p95 < 50 ms sobre 30.000 carpetas; arranque < 1,5 s con caché caliente.

### 3.12 Instalación y actualización → US-8 · SC-006

*Solo en candidatos y estable.*

- Debian/Ubuntu con v1 instalada: `sudo apt update && sudo apt install --only-upgrade nova-flix`.
- Arch con v1 instalada: `sudo pacman -Syu`.
- Windows con v1 instalada: `winget upgrade Novatic.NovaFlix`.

**Esperado**: se actualiza a v2 sin duplicar; el icono y el acceso directo son los nuevos; los datos de v1 siguen ahí.

### 3.13 Verificación de integridad → FR-060

```bash
sha256sum -c SHA256SUMS
```

**Esperado**: todos `OK`.

## 4. Criterios de aprobación

La v2 se considera lista para beta cuando los escenarios 3.1–3.10 pasan en Linux y Windows, y 3.11 y 3.13 pasan en CI. Para el estable, además 3.12.
