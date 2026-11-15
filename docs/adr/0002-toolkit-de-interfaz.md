# ADR-0002 · Toolkit de interfaz: egui/eframe

- **Estado**: Aceptada (revisable)
- **Fecha**: 2026-11-13
- **Relacionado**: Constitución VII · [research.md §2](../../specs/001-novaflix-v2/research.md) · [design-system.md](../../specs/001-novaflix-v2/design-system.md)

## Contexto

La v2 debe ser 100 % Rust para Windows y Linux, sin webview (v1 depende de WebView2 y WebKitGTK), con una interfaz de «aplicación de streaming» (portadas, estanterías, fichas, anillos de progreso) y con un pulido que mezcle lenguajes de Apple y Meta.

## Decisión

Usar **`egui` + `eframe`** con un sistema de diseño propio («Nova Glass»), fuentes Inter incrustadas e iconos Phosphor.

## Razones

- Rust puro, un binario, sin dependencias de webview (NFR-005/006/007).
- Licencia MIT/Apache-2.0: compatible con PolyForm Noncommercial sin obligaciones adicionales.
- Control total del dibujado (degradados, mallas, formas redondeadas) que necesitan las portadas generadas y los anillos.
- Iteración rápida y capturas reproducibles en el entorno de desarrollo.

## Consecuencias

- **No hay desenfoque real del fondo**: el «cristal» se simula con capas translúcidas, degradado y reflejo. Se documenta como limitación (design-system §3.7).
- Hay que construir componentes (tarjeta, estantería, chips, dock, anillo) y el movimiento (`motion.rs`) a mano.
- El texto no soporta *tracking* continuo; los títulos grandes compensan con peso y tamaño.
- El acceso a lectores de pantalla depende de AccessKit.

## Alternativas

| Opción | Por qué no (hoy) |
|---|---|
| Slint | Licencia: GPL-3.0 (incompatible con PolyForm NC) o «Royalty-Free Desktop» con atribución y registro, o comercial |
| Iced | Animación y estilos más costosos, menos widgets |
| GPUI | API inestable; soporte de Windows/Linux reciente |
| Tauri/Dioxus (webview) | Mantienen WebView2/WebKitGTK |
| GTK4/Qt | Distribución pesada en Windows; difícil salirse del tema del sistema |

## Condiciones de revisión

- Si Slint ofreciera una licencia compatible sin atribución obligatoria.
- Si el pulido de `egui` no alcanza los criterios de `design-system.md` tras la fase 4 (se evaluaría `wgpu` con *shaders* propios antes de cambiar de toolkit).
