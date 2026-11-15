# ADR-0003 · Cliente FTP propio, de solo lectura

- **Estado**: Aceptada (revisable)
- **Fecha**: 2026-11-13
- **Relacionado**: Constitución III, IV · [research.md §3–4](../../specs/001-novaflix-v2/research.md) · [contracts/transport.md](../../specs/001-novaflix-v2/contracts/transport.md)

## Contexto

La v2 debe ofrecer FTP además de HTTP, a elección del usuario. El FTP se usa solo para listar y descargar. Además hay que poder salir por un proxy SOCKS5 y manejar servidores tras NAT.

## Decisión

Implementar un cliente FTP mínimo sobre `tokio` (`transport/ftp.rs`) con una **lista cerrada de comandos**: `USER`, `PASS`, `TYPE I`, `OPTS UTF8 ON`, `EPSV`, `PASV`, `CWD`, `MLSD`, `LIST`, `SIZE`, `REST`, `RETR`, `QUIT`. Una sesión por operación. Sin modo activo, sin FTPS, sin escritura.

## Razones

- El subconjunto necesario es pequeño y estable (RFC 959, 2428, 3659).
- La garantía de «solo lectura» (Constitución IV) es **estructural**: los comandos de escritura no existen en el código, y CI lo comprueba.
- Control sobre SOCKS5 en control **y** datos, y sobre las IP privadas anunciadas en `PASV`.
- Evita adaptar el *runtime* de una dependencia a `tokio` (no se verificó cómo opera `suppaftp` en la versión resuelta; ver research §3).

## Consecuencias

- Hay que mantener el análisis de `LIST`/`MLSD` y probar contra servidores reales (rclone y al menos otro).
- Servidores con formatos de `LIST` poco comunes pueden no listarse; el modo Auto cae a HTTP.
- Sin FTPS: las credenciales FTP viajan en claro (el catálogo es público; se avisa en la UI si se usan credenciales).

## Alternativas

| Opción | Motivo de descarte |
|---|---|
| `suppaftp` | Sin SOCKS5 propio; incluye operaciones de escritura; incertidumbre de *runtime* |
| `curl` (libcurl) | Dependencia C pesada; compilación cruzada a Windows más compleja |
| Proceso `rclone` | Exige instalar un binario externo |

## Verificación

- Pruebas de análisis (`ftp::tests`) y de integración contra un servidor FTP real efímero (T-025).
- Comprobación estática en CI: ausencia de `STOR|APPE|DELE|RMD|MKD|RNFR|RNTO|SITE|STOU` (T-028).

## Condición de revisión

Reabrir si `suppaftp` (u otra) cubre SOCKS5, `tokio` y solo-lectura sin necesidad de parches.
