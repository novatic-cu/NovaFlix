# Estrategia de repositorio, versiones, CI/CD y distribución

**Spec**: [spec.md](spec.md) · **Plan**: [plan.md](plan.md) · **ADR**: [0001](../../docs/adr/0001-estrategia-de-repositorio.md), [0005](../../docs/adr/0005-empaquetado-y-distribucion.md)

Este documento parte de un **inventario de lo que v1 ya tiene** (leído del repositorio el 2026-11-13) y define qué se conserva, qué cambia y qué se añade en v2.

## 1. Inventario de v1 (punto de partida)

### 1.1 Repositorio
- Origen: `https://github.com/novatic-cu/NovaFlix.git`, rama `main`, etiqueta `v0.1.0` (2026-10-27).
- Licencia PolyForm Noncommercial 1.0.0 (`LICENSE`, `NOTICE`, `COMMERCIAL.md`).
- Documentación: `README.md`, `CHANGELOG.md` (Keep a Changelog), `CONTRIBUTING.md`, `CODE_OF_CONDUCT.md`, `SECURITY.md`.
- Plantillas: `.github/ISSUE_TEMPLATE/{bug_report,feature_request,config}.yml`, `PULL_REQUEST_TEMPLATE.md`.
- `dependabot.yml`: npm, cargo (`/src-tauri`) y github-actions.
- Sitio: `site/` (GitHub Pages: portada, páginas `apt/` y `pacman/`, tipografías Inter y Instrument Serif).

### 1.2 Flujos de GitHub Actions

| Flujo | Disparador | Qué hace | Estado para v2 |
|---|---|---|---|
| `ci.yml` | push a `main`, PR | Ubuntu 22.04: dependencias WebKitGTK, pnpm, Node, `pnpm build`, `cargo fmt/clippy/test` sobre `src-tauri` | **Reescribir** (Rust puro, Linux + Windows) |
| `release.yml` | etiqueta `v*` o manual | Matriz Linux (`deb,appimage`), Windows (`nsis,msi`), macOS universal (`dmg,app`) con `tauri-action`; release no borrador | **Reescribir** con `cargo-packager`; sin macOS en 2.0; prerelease según etiqueta |
| `apt.yml` | `release: published` o manual | Descarga `*.deb` del release, construye repositorio APT (`dpkg-scanpackages`, `apt-ftparchive`), firma con `APT_GPG_PRIVATE_KEY`/`APT_GPG_PASSPHRASE`, publica en `gh-pages` | **Conservar**; cambiar disparador a `released` |
| `pacman.yml` | `release: published` o manual | Contenedor Arch; descarga `novaflix-bin-<ver>-1-x86_64.pkg.tar.zst`; `repo-add` firmado; publica en `gh-pages` | **Conservar**; el `.pkg.tar.zst` ahora lo produce CI |
| `aur.yml` | `release: published` o manual | Actualiza `PKGBUILD` (versión y SHA-256 del `.tar.gz`), `makepkg --printsrcinfo`, empuja a AUR con `AUR_SSH_PRIVATE_KEY` | **Conservar**; actualizar dependencias del PKGBUILD |
| `codeql.yml` | push/PR a `main`, semanal | Análisis `javascript-typescript` | **Cambiar** a `rust` |
| *(nuevo)* `audit.yml` | PR, semanal | `cargo audit` + `cargo deny` | **Añadir** |
| *(nuevo)* `winget.yml` | `released` | Genera/actualiza manifiestos winget y abre PR | **Añadir** (hoy es manual) |

Secretos existentes: `APT_GPG_PRIVATE_KEY`, `APT_GPG_PASSPHRASE`, `AUR_SSH_PRIVATE_KEY`, `GITHUB_TOKEN`. Clave de firma publicada: `F005 E863 F53A 70D6 0447 9A36 9FE8 FBEF 7C21 10F0`.

### 1.3 Canales y artefactos de v1

| Canal | Artefacto / identificador | Origen |
|---|---|---|
| GitHub Releases | `NovaFlix_0.1.0_amd64.deb`, `NovaFlix_0.1.0_amd64.AppImage`, `NovaFlix_0.1.0_x64-setup.exe` (NSIS), `.msi`, `NovaFlix_0.1.0_universal.dmg` | `tauri-action` |
| GitHub Releases | `novaflix-0.1.0-x86_64.tar.gz` (binario `novaflix`, `novaflix.desktop`, `icons/novaflix-{32,128,256,512}.png`) | **A mano** |
| GitHub Releases | `novaflix-bin-0.1.0-1-x86_64.pkg.tar.zst` | **A mano** |
| APT | Paquete `nova-flix`, repositorio `https://novatic-cu.github.io/NovaFlix/apt` (`stable main`) | `apt.yml` |
| pacman | `novaflix-bin`, repositorio `https://novatic-cu.github.io/NovaFlix/pacman/$arch` | `pacman.yml` |
| AUR | `novaflix-bin` (publicación inicial pendiente: registro de cuentas cerrado) | `aur.yml` |
| winget | `Novatic.NovaFlix` (manifiestos en `packaging/winget/`, publicación manual) | manual |
| Homebrew | `novatic-cu/tap/novaflix` (cask, `packaging/homebrew/`) | manual |

**Hallazgo**: `aur.yml` y `pacman.yml` esperan activos (`.tar.gz`, `.pkg.tar.zst`) que `release.yml` no genera. En v1 se subieron a mano. v2 los automatiza (§5).

## 2. Estrategia de ramas

```
main ─────────●──────────────────────────────●━━━━━━━ (v2 a partir de la beta)
              │ v0.1.0                       ▲ fusión (squash no; merge commit)
v1  ──────────●────●────(parches 0.1.x)      │
              └─ rama de mantenimiento       │
v2/rust-rewrite ──●──●──●──●──●──────────────┘
```

| Rama | Propósito | Reglas |
|---|---|---|
| `main` | Hasta la beta: sigue siendo v1 publicable. Desde la fusión: v2. | Protegida; PR con CI en verde; sin *force push* |
| `v1` | Mantenimiento de v1 (`0.1.x`). Creada desde `main@899cc24`. | Solo correcciones críticas de seguridad; etiquetas `v0.1.N` |
| `v2/rust-rewrite` | Reescritura. | PR internos por fase; la fusión a `main` se hace con *merge commit* para conservar el historial de la reescritura |
| `feat/*`, `fix/*`, `docs/*` | Ramas de trabajo (CONTRIBUTING) | Conventional Commits |

**Momento de la fusión**: al publicar `v2.0.0-beta.1`, con CI verde en Linux y Windows, SC-001…SC-005 cumplidos y la migración verificada con datos reales. Hasta entonces `main` no se toca.

**Protección de v1**: la etiqueta `v0.1.0` y la rama `v1` no se mueven. Un futuro `v0.1.1` se etiqueta desde `v1`.

## 3. Versionado

SemVer. Etiquetas `vMAJOR.MINOR.PATCH[-prerelease]`.

| Fase | Etiquetas | ¿Dispara canales? |
|---|---|---|
| Alfas | `v2.0.0-alpha.1`, `-alpha.2`… | Solo GitHub Releases (marcado como *prerelease*) |
| Betas | `v2.0.0-beta.1`… | Solo GitHub Releases (*prerelease*) |
| Candidato | `v2.0.0-rc.1` | Solo GitHub Releases (*prerelease*) |
| Estable | `v2.0.0` | **Todos** los canales |

- La versión única vive en `[workspace.package].version` de `Cargo.toml`; ambos crates la heredan.
- Un script de comprobación en CI verifica que la etiqueta coincide con la versión del workspace.
- `CHANGELOG.md` se actualiza en el mismo PR que sube la versión.

**Por qué las prereleases no llegan a APT/pacman/AUR**
1. `release: [published]` se dispara también con *prereleases*; hay que usar `released` (que no lo hace).
2. En Arch, `pkgver` no admite guion: `2.0.0-alpha.1` es inválido.
3. Un `apt upgrade` de usuarios de v1 no debe instalar una alfa.

## 4. CI (`ci.yml`)

```
on: push (main, v1), pull_request
concurrency: cancel-in-progress por ref
jobs:
  lint:     ubuntu-latest   → cargo fmt --check; cargo clippy --workspace --all-targets -D warnings
  test:     matrix {ubuntu-latest, windows-latest}
                            → cargo test --workspace   (incluye integración FTP/HTTP locales)
  audit:    ubuntu-latest   → cargo audit; cargo deny check
  specs:    ubuntu-latest   → comprobación de que tasks.md no tiene tareas [x] sin evidencia (script)
  forbidden-ftp: ubuntu     → grep -E 'STOR|APPE|DELE|RMD|MKD|RNFR|RNTO|SITE|STOU' crates/novaflix-core/src/transport/ftp.rs  ⇒ debe fallar si hay coincidencias
  perf:     ubuntu-latest   → prueba de búsqueda p95 y arranque (umbral NFR-001/002)
```

- Dependencias del sistema en Linux para compilar `egui`: `libxkbcommon-dev`, `libwayland-dev`, `libgl1-mesa-dev`, `libxcb*-dev`, `libfontconfig1-dev` (a confirmar al compilar en CI).
- Caché con `swatinem/rust-cache@v2`.
- Toolchain `dtolnay/rust-toolchain@stable` con `clippy` y `rustfmt`; el MSRV (1.95) se verifica en un job aparte `msrv`.

## 5. Release (`release.yml`)

**Disparador**: etiqueta `v*` o `workflow_dispatch`.

### 5.1 Matriz

| Plataforma | Formatos | Herramienta |
|---|---|---|
| `ubuntu-22.04` | `.deb`, `.AppImage`, `.tar.gz`, `.pkg.tar.zst` | `cargo-packager` + script de tarball + `makepkg` en contenedor Arch |
| `windows-latest` | NSIS `.exe`, `.msi` | `cargo-packager` (NSIS, WiX) |

> macOS queda fuera de 2.0 (spec §3.2, C-7).

### 5.2 Nombres de artefacto (**contrato congelado**)

Los flujos APT, pacman, AUR y winget dependen de estos nombres. No cambiar sin actualizarlos. `cargo-packager` produce nombres propios; el flujo de release los **renombra** a los de v1.

| Artefacto | Lo que genera `cargo-packager` (verificado en Linux, 2026-11-14) | Nombre publicado | Consumido por |
|---|---|---|---|
| Paquete Debian | `novaflix_<ver>_amd64.deb` con `Package: nova-flix` | `NovaFlix_<ver>_amd64.deb` | `apt.yml` (usa `dpkg-deb -f Package`: lo crítico es el **nombre del paquete**, comprobado en el flujo con `grep -x 'Package: nova-flix'`) |
| AppImage | `novaflix_<ver>_x86_64.AppImage` | `NovaFlix_<ver>_amd64.AppImage` | README |
| Tarball | *(script `make-tarball.sh`)* | `novaflix-<ver>-x86_64.tar.gz` con `novaflix`, `novaflix.desktop`, `icons/novaflix-{16,22,24,32,48,64,128,256,512}.png` | `aur.yml`, `PKGBUILD` |
| Paquete Arch | *(`makepkg` con `packaging/arch/PKGBUILD`; verificado en local)* | `novaflix-bin-<ver>-1-x86_64.pkg.tar.zst` (solo versiones estables) | `pacman.yml` |
| NSIS | *(sin verificar: requiere Windows)* `*setup.exe` | `NovaFlix_<ver>_x64-setup.exe` | Manifiesto winget |
| MSI | *(sin verificar)* `*.msi` | `NovaFlix_<ver>_x64_en-US.msi` | Descarga manual |
| Sumas | — | `SHA256SUMS` (todos los anteriores) | Verificación de usuarios |

`<ver>` es la versión sin el prefijo `v`. Los nombres de NSIS/MSI se resolverán con comodines (`*setup.exe`, `*.msi`) hasta que se verifiquen en el primer release en `windows-latest` (T-079).

### 5.3 Pasos

1. `checkout`, toolchain, caché.
2. Comprobar etiqueta = versión del workspace (falla si no).
3. `cargo build --release -p novaflix` (LTO, `strip`).
4. `cargo packager --release` con los formatos de la plataforma.
5. Linux: construir `tar.gz`; `pkg.tar.zst` con `makepkg` dentro de `archlinux:latest` usando un `PKGBUILD` de empaquetado local (no el del AUR).
6. Generar `SHA256SUMS`.
7. Publicar el release (`softprops/action-gh-release`), con `prerelease: ${{ contains(github.ref_name, '-') }}`, cuerpo desde la sección correspondiente de `CHANGELOG.md`, y adjuntar los artefactos.
8. (Opcional, 2.1) Atestación de procedencia con `actions/attest-build-provenance`.

### 5.4 Cuerpo del release
Generado desde `CHANGELOG.md` (extrae la sección de la versión) más una tabla de instaladores y la advertencia de SmartScreen mientras no haya firma Authenticode (C-8). Desaparece la advertencia de macOS.

## 6. Publicación a canales

| Flujo | Cambio respecto a v1 |
|---|---|
| `apt.yml` | `on: release: types: [released]`. Sin cambios en el resto. Verifica que `Package` sea `nova-flix` |
| `pacman.yml` | Ídem `released`. El `.pkg.tar.zst` proviene del release |
| `aur.yml` | Ídem `released`. `PKGBUILD`: `depends=('libxkbcommon' 'libxkbcommon-x11' 'wayland' 'libglvnd' 'libx11' 'libxcursor' 'libxi' 'dbus' 'hicolor-icon-theme')` (sin WebKitGTK) y `optdepends=('vlc: reproductor' 'mpv: reproductor')` |
| `winget.yml` (nuevo) | Con `released`: calcula SHA-256 del NSIS y abre PR contra `microsoft/winget-pkgs` usando `vedantmgoyal9/winget-releaser` o equivalente |
| Homebrew | Congelado en `0.1.0` hasta decidir macOS |

**Orden y bloqueo**: `apt` y `pacman` comparten el grupo de concurrencia `gh-pages-publish` (`cancel-in-progress: false`) — **no cambiar**: v1 tuvo una carrera por el *lock* de `gh-pages` (`e8ced05`).

**Actualización v1 → v2 (SC-006)**: el mismo nombre de paquete `nova-flix`/`novaflix-bin` y `Novatic.NovaFlix` con versión mayor (`2.0.0 > 0.1.0`) hace que los gestores actualicen en lugar de duplicar. Debe probarse en el candidato `rc`.

## 7. Empaquetado

Configuración en `[package.metadata.packager]` de `crates/novaflix-app/Cargo.toml` (a completar en T-060):

| Campo | Valor |
|---|---|
| `product-name` | `NovaFlix` |
| `identifier` | `com.novatic.novaflix` |
| `version` | heredada del workspace |
| `description` | «Explorador de catálogos multimedia públicos» |
| `authors` | `Novatic` |
| `license-file` | `LICENSE` |
| `icons` | `assets/icons/{32x32.png,128x128.png,128x128@2x.png,icon.icns,icon.ico}` |
| `formats` | `deb`, `appimage`, `nsis`, `wix` |
| Debian `depends` | `libegl1`, `libgl1`, `libxkbcommon0`, `libxkbcommon-x11-0`, `libwayland-client0`, `libwayland-egl1`, `libx11-6`, `libx11-xcb1`, `libxcursor1`, `libxi6`, `libdbus-1-3` — **verificado**: `ldd` solo muestra `libc`, `libgcc_s` y `libm`; el resto se carga con `dlopen` (lista obtenida de las cadenas del binario) |
| `.desktop` | `Name=NovaFlix`, `Exec=novaflix %U`, `Icon=novaflix`, `Categories=AudioVideo;Player;Utility;`, `StartupWMClass=novaflix` |
| `app_id` (Wayland) | `com.novatic.novaflix` (debe coincidir con el `.desktop` para que el icono salga bien) |

## 8. Seguridad de la cadena de suministro

- `cargo audit` y `cargo deny` (licencias permitidas: MIT, Apache-2.0, BSD, ISC, OFL-1.1, Unicode, MPL-2.0; sin GPL) en cada PR y semanalmente.
- Dependabot: `cargo` en `/` (workspace), `github-actions` mensual; se elimina `npm`.
- CodeQL con `rust`.
- Acciones fijadas por versión mayor (`@v4`); revisión trimestral.
- Firma: repositorios APT/pacman con la clave existente; `SHA256SUMS` en cada release. Firma Authenticode pendiente (C-8).
- Permisos mínimos por flujo (`contents: read` salvo los que publican).

## 9. Documentación del repositorio que cambia

| Archivo | Cambio |
|---|---|
| `README.md` | Nueva descripción, capturas, tabla de protocolos, instalación sin WebView2, sección «Detrás de un proxy», desarrollo con `cargo` |
| `CONTRIBUTING.md` | Entorno Rust (sin Node/pnpm), estilo egui, SDD (`specs/`), pruebas |
| `CHANGELOG.md` | Sección `[2.0.0-alpha.1]` |
| `SECURITY.md` | Añadir el modelo de amenazas del transporte |
| `.gitignore` | `target/`, `*.part`, artefactos de empaquetado |
| `site/` | Portada de v2 y actualización de las páginas APT/pacman |
| `.github/ISSUE_TEMPLATE` | Campos nuevos: protocolo (HTTP/FTP), «¿detrás de proxy?», pegar diagnóstico |

## 10. Lista de verificación de release

- [ ] `CHANGELOG.md` actualizado y fecha puesta
- [ ] Versión del workspace = etiqueta
- [ ] CI en verde (lint, test Linux/Windows, audit, perf)
- [ ] `quickstart.md` recorrido en Linux y Windows
- [ ] Migración probada con datos reales de v1
- [ ] Artefactos con los nombres del §5.2 y `SHA256SUMS`
- [ ] (solo estable) `apt`, `pacman`, `aur`, `winget` en verde
- [ ] (solo estable) `apt upgrade` y `pacman -Syu` desde v1 verificados
- [ ] Release no borrador; *prerelease* correcto
