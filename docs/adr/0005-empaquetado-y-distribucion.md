# ADR-0005 · Empaquetado y distribución sin Tauri

- **Estado**: Aceptada, pendiente de verificación (T-061)
- **Fecha**: 2026-11-13
- **Relacionado**: Constitución VIII · [research.md §7](../../specs/001-novaflix-v2/research.md) · [release-and-ci.md](../../specs/001-novaflix-v2/release-and-ci.md)

## Contexto

`tauri-action` producía `.deb`, `.AppImage`, NSIS, MSI y DMG en v1. Sin Tauri hay que obtener los mismos formatos. Además se descubrió que el `.tar.gz` (AUR) y el `.pkg.tar.zst` (pacman) de v1 se subieron a mano, lo que contradice la Constitución VIII.

## Decisión

1. Usar **`cargo-packager`** para `.deb`, `.AppImage`, NSIS y MSI (WiX).
2. Generar con scripts en CI el `.tar.gz`, el `.pkg.tar.zst` (con `makepkg` en un contenedor `archlinux`) y `SHA256SUMS`.
3. **Congelar los nombres de artefacto** (release-and-ci §5.2) y el nombre de paquete Debian `nova-flix`.
4. Cambiar `apt.yml`, `pacman.yml` y `aur.yml` a `release: types: [released]` para que las prereleases no se publiquen en los canales.

## Razones

- `cargo-packager` nace del empaquetador de Tauri y apunta a binarios que no son Tauri, con los mismos formatos.
- Automatizar los artefactos de Arch elimina un paso manual y los errores de versión/SHA.
- `released` no se dispara con prereleases (sí lo hace `published`); además `pkgver` de Arch no admite guiones.

## Consecuencias

- Hay que verificar que los nombres reales coinciden con los congelados y, si no, renombrar en el flujo.
- `cargo-packager` es más joven; se prueban `deb` y `AppImage` localmente antes de etiquetar.
- Linux ya no depende de WebKitGTK: cambian las dependencias del `PKGBUILD` y del `.deb`.

## Alternativas

| Opción | Motivo |
|---|---|
| `cargo-deb` + `cargo-wix` + `linuxdeploy` | Más control, más piezas |
| `cargo-bundle` | Soporte irregular en Windows |
| Inno Setup | Fuera del ecosistema Rust |

## Verificación

- T-061: `cargo packager` genera `deb` y `appimage`; `dpkg-deb -f … Package` = `nova-flix`.
- T-079: la etiqueta `v2.0.0-alpha.1` crea un release *prerelease* y **no** dispara `apt`/`pacman`/`aur`.
