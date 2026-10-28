# Política de seguridad

## Versiones soportadas

| Versión | Soporte |
|---|---|
| 0.1.x | ✅ |

## Reportar una vulnerabilidad

No abras un issue público. Usa **Security → Report a vulnerability** en GitHub (Private Vulnerability Reporting) o escribe a **marcosjaviersantana300302@gmail.com**.

Incluye:

1. Versión y plataforma.
2. Pasos para reproducir.
3. Impacto esperado (confidencialidad, integridad, disponibilidad).
4. Prueba de concepto si la tienes.

Compromiso de respuesta: acuse en 72 h y evaluación inicial en 7 días. Si el reporte es válido, se publica un parche y se te acredita salvo que prefieras lo contrario.

## Alcance

Dentro de alcance:

- Ejecución de código o escape del sandbox del webview.
- Path traversal al descargar archivos.
- Inyección SQL/FTS en el backend.
- Fuga de datos personales (`app.db`).
- SSRF fuera del host permitido.

Fuera de alcance:

- Ataques que requieran control físico del equipo.
- Contenido alojado en el catálogo de terceros.
- Falta de firma/notarización de las builds de macOS.
- Ingeniería social.

## Diseño de seguridad

- CSP estricta y `allowlist` de un único host para el backend.
- Sin telemetría ni llamadas a servicios externos.
- Descargas con nombre saneado, escritura en `.part` y renombrado atómico.
- SQLite local con consultas parametrizadas.
- Dependabot + CodeQL activos en el repositorio.

## Riesgos aceptados

| Dependencia | Aviso | Motivo |
|---|---|---|
| `glib 0.18.x` (transitiva vía `gtk 0.18` → `tauri 2.12`) | RUSTSEC de insonoridad en `VariantStrIter` | NovaFlix no usa `VariantStrIter` ni recibe variantes GVariant de entrada; el fallo vive en el binding GTK. No hay versión parcheada en la rama 0.18 y actualizar a 0.20 exige que `tauri` migre de gtk-rs. Se revisará en cada actualización de Tauri. |
