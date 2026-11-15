# Marca e icono de NovaFlix v2

**Spec**: [spec.md](spec.md) · **Diseño**: [design-system.md](design-system.md)

## 1. Punto de partida (v1)

Icono actual (`assets/logo.svg`): cuadrado de esquinas redondeadas (`rx=232`) con degradado azul marino → azul (`#0B1220 → #122A4C → #0A84FF`), una «N» blanca de barras planas y una chispa de cuatro puntas arriba a la derecha. Es limpio pero genérico: la «N» plana se confunde con muchos iconos, no tiene profundidad y no comunica «vídeo».

**Se conserva**: la «N», el azul `#0A84FF` como color de marca y la chispa (herencia visual de v1).
**Se cambia**: forma (cuadrado continuo estilo Apple), profundidad (capas de cristal), y la diagonal de la «N», que pasa a ser una cinta translúcida con un destello de reproducción.

## 2. Concepto

> **«N» de cristal sobre noche azul.** Dos pilares sólidos y una diagonal de cristal que los cruza; entre ambos queda la luz. La diagonal lleva un pequeño triángulo de reproducción tallado en su interior.

| Elemento | Decisión |
|---|---|
| Forma | *Squircle* de curvatura continua (superelipse, n ≈ 5), 1024×1024, sin transparencia en el icono de aplicación |
| Fondo | Degradado diagonal `#070B1A` (arriba izquierda) → `#12307A` (centro) → `#0A84FF` (abajo derecha), con un velo `#5E5CE6` al 35 % en la esquina inferior derecha |
| Luz | Resplandor radial `#2E7DFF` al 45 % detrás del glifo; reflejo superior (arco blanco 14 % → 0 %) |
| Pilares | Blanco → `#CFE3FF` (degradado vertical), bordes con radio 14 |
| Diagonal | Cristal: degradado `#BFE0FF` 90 % → `#5AB0FF` 70 %, con borde interior blanco 35 % y *multiply* donde cruza los pilares para sugerir transparencia |
| Destello de reproducción | Triángulo redondeado de reproducción en el tercio central de la diagonal, en `#0A84FF` al 85 % |
| Chispa | Cuatro puntas, `#B9DCFF → #5AB0FF`, arriba a la derecha (más pequeña que en v1) |
| Sombra | Sombra suave bajo el glifo (`0 24 48 #02060F` 45 %) para separarlo del fondo |
| Borde | Trazo interior de 2 px blanco al 10 % siguiendo el *squircle* (acabado de «vidrio») |

**Por qué funciona**: silueta reconocible a 16 px (la «N» sigue legible aunque se pierdan los detalles); profundidad que la separa de los iconos planos; el destello de reproducción dice «vídeo» sin copiar a nadie; el azul vibrante enlaza con los botones de la interfaz (design-system §3.1).

**Qué evita**: el rojo y la «N» en cinta de Netflix; la «∞» de Meta; el logotipo de Apple. La «N» es tipográfica y propia.

## 3. Variantes

| Archivo | Uso | Detalle |
|---|---|---|
| `assets/logo.svg` | Maestro, 1024×1024 | Todas las capas |
| `assets/logo-small.svg` | 16–48 px | Sin chispa ni sombra; pilares y diagonal engrosados un 12 %; sin triángulo de reproducción (se perdería) |
| `assets/logo-mono.svg` | Bandeja, notificaciones, icono simbólico de Linux | Solo la «N», un color, sin degradado |
| `assets/logo-wordmark.svg` | Portada, README, sitio | Icono + «NovaFlix» en Inter 600 |
| `assets/social-preview.svg/.png` | Vista previa en redes (1280×640) | Se regenera con el nuevo icono |

### 3.1 Tamaños derivados

| Plataforma | Archivos | Origen |
|---|---|---|
| Linux (hicolor) | `16, 22, 24, 32, 48, 64, 128, 256, 512` px en `packaging/icons/hicolor/<n>x<n>/apps/novaflix.png` | Tamaños ≤ 48 desde `logo-small.svg` |
| AUR / tarball | `icons/novaflix-{32,128,256,512}.png` | Mismos archivos |
| Windows | `icon.ico` con 16, 20, 24, 32, 40, 48, 64, 128, 256 | Mismas fuentes; PNG comprimido en la entrada de 256 |
| Ventana | `icon.png` 256 incrustado en el binario (`include_bytes!`) | Para `egui`/`winit` (`with_icon`) |
| macOS (futuro) | `icon.icns` | Reservado (C-7) |

## 4. Reglas de uso

- **Zona de seguridad**: margen mínimo = 1/8 del lado.
- **Tamaño mínimo**: 16 px (usar `logo-small`).
- **Fondo**: el icono de aplicación lleva su propio fondo; sobre fondos claros u oscuros se coloca tal cual, sin recolorear.
- **No** estirar, rotar, cambiar los colores ni añadir sombras externas.
- **Texto de marca**: «NovaFlix» (una palabra, N y F en mayúscula). Nunca «Nova Flix» salvo el nombre del paquete APT `nova-flix`.

## 5. Paleta de marca

| Nombre | Hex | Uso |
|---|---|---|
| Noche | `#070B1A` | Fondo del icono, base de la interfaz |
| Azul Nova | `#0A84FF` | Color de marca y acción primaria |
| Índigo | `#5E5CE6` | Final del degradado de acento |
| Hielo | `#CFE3FF` | Glifo |
| Cian suave | `#5AB0FF` | Chispa y cristal |

## 6. Criterios de aceptación del icono

- [ ] Legible y reconocible como «N» a 16, 24 y 32 px sobre fondos claro y oscuro (revisión con capturas de ambos).
- [ ] Sin artefactos de borde (halo) al convertir a PNG con fondo transparente en las esquinas del *squircle*.
- [ ] El `.ico` contiene todas las resoluciones del §3.1 (`magick identify`).
- [ ] Contraste del glifo contra el fondo ≥ 4,5 : 1 en el centro (verificación visual + muestreo de píxeles).
- [ ] Se ve bien en el selector de aplicaciones, la barra de tareas y la lista de Alt+Tab (verificación manual en Linux; en Windows durante la validación del instalador).
- [ ] El SVG maestro pesa < 8 KiB y no usa filtros no portables entre `rsvg-convert` y navegadores.

## 7. Generación reproducible

Un script (`scripts/make-icons.sh`) genera todos los derivados a partir de los SVG con `rsvg-convert` y `magick`; es **idempotente** y se documenta en `CONTRIBUTING.md`. Los derivados se versionan (no se generan en CI) para que los instaladores sean reproducibles sin depender de ImageMagick.
