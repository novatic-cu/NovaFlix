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


## Modelo de amenazas del transporte (v2)

| Amenaza | Mitigación |
|---|---|
| Un servidor redirige a otro host (robo de sesión, contenido ajeno) | Solo se siguen redirecciones al mismo host del catálogo; prueba `redirects_to_other_hosts_are_rejected` |
| Nombre de archivo remoto con `../`, separadores o caracteres de control | `sanitize_file_name` y destino siempre dentro de la carpeta elegida; prueba `ct07_hostile_names_never_escape_the_download_dir` |
| Un cliente FTP comprometido o modificado escribe en el servidor | El cliente solo implementa comandos de lectura; `scripts/check-ftp-readonly.sh` falla en CI si aparece uno de escritura |
| Fuga de credenciales del proxy | No se guardan en disco ni en la base de datos; el diagnóstico y las etiquetas las enmascaran (`proxy_credentials_are_masked`, `ct05`) |
| Un servidor FTP acepta `REST` pero reenvía el fichero entero (corrupción silenciosa) | El flujo detecta que excede `total − offset`, y la descarga verifica el tamaño final y reinicia desde cero |
| TLS | Siempre verificado (rustls con raíces del sistema); no existe opción para desactivarlo |
