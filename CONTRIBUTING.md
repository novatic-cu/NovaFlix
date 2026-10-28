# Contribuir a NovaFlix

Gracias por tomarte el tiempo. Este documento resume cómo preparar el entorno, qué estilo seguir y qué esperar de una revisión.

## Entorno

```bash
pnpm install
pnpm tauri dev
```

Requisitos:

- Node 24 y pnpm 12 (Corepack los activa desde `package.json`).
- Rust estable 1.77+.
- Linux: `webkit2gtk-4.1`, `gtk3`, `libssl`, `libayatana-appindicator3`, `librsvg`.
- Windows: WebView2 + MSVC Build Tools.

## Flujo de trabajo

1. Abre un issue describiendo el problema o la propuesta antes de cambios grandes.
2. Crea una rama: `feat/<tema>`, `fix/<tema>`, `docs/<tema>`.
3. Commits en estilo [Conventional Commits](https://www.conventionalcommits.org/): `feat:`, `fix:`, `docs:`, `refactor:`, `perf:`, `test:`, `chore:`.
4. Abre un pull request contra `main` con una descripción clara y capturas si toca UI.

## Estándares de código

### Rust

- `cargo fmt` y `cargo clippy -- -D warnings` deben pasar.
- Errores como `Result<T, String>` en la frontera IPC; sin `unwrap()` en rutas de usuario.
- Las operaciones de red no bloquean el hilo de UI: `tokio` + `spawn`.

### TypeScript / React

- Componentes de función, exports nombrados, `'use client'` no aplica (no hay RSC).
- Estado global solo en `src/store.ts` (Zustand); el resto es estado local.
- Tipos en `src/lib/types.ts`, mapeados 1:1 con los `struct` de Rust.
- Sin colores literales: usa los tokens de `src/styles/index.css`.

### UI

Estilo plano corporativo con inspiración Apple HIG:

- Una sola superficie por modal; nada de tarjeta dentro de tarjeta.
- Jerarquía por tipografía, espaciado y divisores de 1 px.
- Movimiento con springs (`damping 1.0`, `response 0.3`); sin `@keyframes` en interacciones arrastrables.
- Respeta `prefers-reduced-motion`, `prefers-reduced-transparency` y `prefers-contrast`.

## Pruebas

```bash
pnpm build                      # TypeScript + bundle
cargo test --manifest-path src-tauri/Cargo.toml
cargo clippy --manifest-path src-tauri/Cargo.toml -- -D warnings
```

Si añades lógica de normalización, parsers o esquema, incluye pruebas unitarias.

## Licencia de las contribuciones

Al enviar un pull request aceptas que tu contribución se publique bajo la licencia del proyecto (PolyForm Noncommercial 1.0.0) y que el titular pueda ofrecer licencias comerciales que la incluyan.

## Código de conducta

Toda participación se rige por [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).
