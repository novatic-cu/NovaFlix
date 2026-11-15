# ADR-0001 · Estrategia de repositorio para la v2

- **Estado**: Aceptada
- **Fecha**: 2026-11-13
- **Decide**: propietario del proyecto (delegado en el asistente de desarrollo, a revisar)
- **Relacionado**: Constitución IX · [research.md §1](../../specs/001-novaflix-v2/research.md) · [release-and-ci.md §2](../../specs/001-novaflix-v2/release-and-ci.md)

## Contexto

NovaFlix v1 (`v0.1.0`, Tauri + React) está publicada por APT (`nova-flix`), pacman y AUR (`novaflix-bin`), winget (`Novatic.NovaFlix`) y GitHub Pages, todo ligado al repositorio `novatic-cu/NovaFlix` y a una clave de firma pública. La v2 es una reescritura íntegra en Rust. Hay que decidir dónde vive el código y cómo se conserva la v1.

## Decisión

1. Mantener el repositorio `novatic-cu/NovaFlix`.
2. Crear la rama **`v1`** desde `main@899cc24` (conserva la v1; recibirá solo parches críticos `0.1.x`). La etiqueta `v0.1.0` no se mueve.
3. Desarrollar la v2 en **`v2/rust-rewrite`**; fusionar en `main` (merge commit) al publicar `v2.0.0-beta.1`.
4. Las prereleases (`alpha`, `beta`, `rc`) no se publican en APT/pacman/AUR/winget.

## Consecuencias

**Positivas**
- Las instalaciones existentes se actualizan (mismo nombre de paquete, versión mayor).
- Se conservan issues, estrellas, enlaces y la clave de firma.
- La v1 es recuperable y parcheable sin mezclarse con la v2.

**Negativas / costes**
- Dos cadenas de herramientas conviven un tiempo (Node/Tauri en `v1`, Cargo en `v2/…`); `main` debe seguir construyendo v1 hasta la fusión.
- Los flujos de publicación deben endurecerse (`released` en lugar de `published`) para que una alfa no llegue a los usuarios.

## Alternativas consideradas

| Alternativa | Motivo de descarte |
|---|---|
| Repositorio nuevo `NovaFlix-v2` | Rompe actualizaciones y fragmenta la comunidad |
| Monorepo `v1/` + `v2/` en `main` | Historial confuso, CI con dos cadenas de herramientas en cada commit |
| Sustituir `main` de inmediato | Deja `main` sin v1 publicable y dispara los flujos de publicación con un binario incompleto |

## Verificación

- `git branch -vv` muestra `v1` y `v2/rust-rewrite` apuntando al mismo commit de partida (hecho).
- Tras la fusión, `git log v1..main` contiene solo la reescritura.
