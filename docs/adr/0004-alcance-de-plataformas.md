# ADR-0004 · Alcance de plataformas: Linux y Windows

- **Estado**: Aceptada
- **Fecha**: 2026-11-13
- **Relacionado**: spec §3.2 · C-7

## Contexto

La v1 distribuye también un `.dmg` universal de macOS y un cask de Homebrew. La petición de la v2 fue «para Windows y Linux».

## Decisión

La v2.0 soporta **Linux x86_64** y **Windows 10/11 x86_64**. macOS queda fuera de 2.0; el cask de Homebrew se congela en `0.1.0`.

## Razones

- Es lo que se pidió; cada plataforma añadida multiplica la verificación (no hay macOS disponible para pruebas en el entorno de desarrollo).
- `egui` y `cargo-packager` permitirían macOS más adelante sin rediseño.

## Consecuencias

- `release.yml` no incluye macOS; `README` indica que macOS usa v1 (`0.1.0`).
- Se evita publicar un binario de macOS sin probar.

## Revisión

Reabrir con la aclaración C-7 si hay demanda. Coste estimado: un *job* en `release.yml`, `icon.icns`, actualización del cask y pruebas manuales en macOS.
