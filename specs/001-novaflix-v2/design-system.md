# Sistema de diseño: «Nova Glass»

**Spec**: [spec.md](spec.md) · **Pantallas**: [contracts/ui-screens.md](contracts/ui-screens.md) · **Marca**: [branding.md](branding.md) · **Toolkit**: `egui` (ADR-0002)

## 1. Intención

NovaFlix v2 tiene que **parecer lo que es**: una aplicación para ver qué hay y bajarlo, no una herramienta de transferencia de archivos. Debe sentirse como una aplicación de streaming moderna, no como FileZilla ni como el buscador plano de v1.

El lenguaje visual mezcla dos escuelas con roles claros:

| Escuela | Qué se toma | Dónde se nota |
|---|---|---|
| **Apple** (HIG, tvOS/macOS/visionOS) | Restricción, jerarquía por tipografía y espacio, capas translúcidas con profundidad, esquinas concéntricas, movimiento con muelles e interrumpible, contenido como protagonista | Tipografía, portadas, fichas, transiciones, densidad baja |
| **Meta** (Quest, Facebook, Instagram) | Botones en cápsula con azul vibrante y degradado, dock flotante con iconos, tarjetas muy redondeadas, microinteracciones lúdicas y claras, acciones primarias evidentes | Navegación lateral flotante, botones, chips, anillos de progreso, estados vacíos |

**Regla de oro**: el contenido manda. El cromado (barras, paneles) se retira o se vuelve translúcido; los títulos y portadas ocupan el espacio.

**Anti-objetivos**: no parecerse a un cliente FTP (nada de paneles dobles, árboles de carpetas ni tablas densas en las pantallas principales); no copiar marcas ni activos de terceros (ni el rojo de Netflix, ni el logotipo de Meta, ni SF Pro).

## 2. Principios de diseño

1. **Portada primero.** Cada título es una tarjeta con identidad visual (portada generada); las listas de texto se reservan para archivos dentro de una ficha.
2. **Una acción primaria por pantalla**, en cápsula azul; el resto, secundarias translúcidas o texto.
3. **Profundidad por capas**, no por sombras duras: fondo → superficie → superficie elevada → vidrio.
4. **Movimiento con significado**: aparece lo que entra, se desplaza lo que cambia de lugar; nada se anima por decorar. Siempre interrumpible y desactivable.
5. **Estados completos**: cada pantalla define cargando, vacío, error y éxito (contracts/ui-screens.md).
6. **Honestidad de red a la vista**: la barra de estado muestra el protocolo y el proxy; los errores explican y proponen.
7. **Accesible de serie**: contraste AA, foco visible, teclado completo, zoom, reducir movimiento.

## 3. Tokens

Los tokens se definen una sola vez en `theme.rs` y no se usan colores literales fuera de él.

### 3.1 Color — tema oscuro (por defecto)

| Token | Valor | Uso |
|---|---|---|
| `bg.base` | `#07090F` | Fondo de la ventana |
| `bg.surface` | `#0E121C` | Paneles, barra lateral |
| `bg.raised` | `#151A27` | Tarjetas, campos |
| `bg.overlay` | `#1C2233` | Fichas, menús |
| `glass.fill` | blanco 7 % | Capas de cristal sobre contenido |
| `glass.fill.hover` | blanco 11 % | Estado *hover* del cristal |
| `glass.stroke` | blanco 12 % | Borde de 1 px del cristal |
| `text.primary` | `#F5F7FB` | Títulos y cuerpo principal |
| `text.secondary` | `#A9B1C3` | Cuerpo secundario |
| `text.tertiary` | `#7C869B` | Metadatos, ayudas |
| `accent` | `#0A84FF` | Acción primaria, foco, selección |
| `accent.hover` | `#3D9BFF` | *Hover* de acción primaria |
| `accent.2` | `#5E5CE6` | Final del degradado de acento |
| `accent.fill` | `#0A70E0` | Inicio del relleno de botones primarios (texto blanco ≥ 4,77 : 1) |
| `accent.fill.2` | `#5E5CE6` | Final del relleno de botones primarios (texto blanco ≥ 5,06 : 1) |
| `accent.gradient` | `accent.fill → accent.fill.2` a 135° | Botón primario, anillos, portada destacada |
| `success` | `#30D158` | Descarga completa, ✓ de diagnóstico |
| `warning` | `#FFD60A` | Avisos |
| `danger` | `#FF453A` | Errores, ✕ de diagnóstico |
| `focus.ring` | `accent` al 70 %, 2 px + 2 px de separación | Foco de teclado |

### 3.2 Color — tema claro

| Token | Valor |
|---|---|
| `bg.base` | `#F4F6FA` |
| `bg.surface` | `#FFFFFF` |
| `bg.raised` | `#FFFFFF` (con borde `#E3E7EF`) |
| `bg.overlay` | `#FFFFFF` (con sombra suave) |
| `glass.fill` | negro 4 % |
| `glass.stroke` | negro 8 % |
| `text.primary` | `#0B1220` |
| `text.secondary` | `#475069` |
| `text.tertiary` | `#667189` |
| `accent` | `#0A6CDB` (más oscuro para contraste sobre blanco) |

### 3.3 Contraste (verificado por prueba automática)

| Par | Ratio mínimo | Requisito |
|---|---|---|
| `text.primary` sobre `bg.base` / `bg.surface` / `bg.raised` | ≥ 4,5 : 1 | AA texto normal |
| `text.secondary` sobre `bg.surface` | ≥ 4,5 : 1 | AA |
| `text.tertiary` sobre `bg.surface` | ≥ 3 : 1 | Solo metadatos de ≥ 13 px (AA texto grande/UI) |
| Texto blanco sobre `accent.fill` / `accent.fill.2` | ≥ 4,5 : 1 | AA en botones |
| `danger`, `success` sobre `bg.surface` | ≥ 3 : 1 | Iconos y estados (no solo color: llevan icono) |

Una prueba unitaria (`theme::tests::contrast`) calcula la luminancia relativa WCAG de cada par y falla si baja del umbral (SC-008).

### 3.4 Tipografía

| Familia | Uso | Pesos |
|---|---|---|
| **Inter** | Todo el texto | 400, 500, 600 |
| **Inter Tight** *(opcional)* | Títulos grandes si se incorpora | 600 |
| Fuente monoespaciada del sistema | Rutas, atajos, diagnóstico | 400 |

Escala (px a zoom 1.0) — `egui` no admite *tracking* continuo; los títulos grandes se compensan con peso y tamaño:

| Rol | Tamaño / interlineado | Peso | Uso |
|---|---|---|---|
| `display` | 40 / 46 | 600 | Título de la portada destacada |
| `title1` | 28 / 34 | 600 | Título de pantalla y de ficha |
| `title2` | 22 / 28 | 600 | Cabecera de estantería |
| `headline` | 17 / 22 | 600 | Título de tarjeta, filas importantes |
| `body` | 15 / 22 | 400 | Texto general |
| `callout` | 13 / 18 | 500 | Etiquetas, chips, botones |
| `caption` | 11.5 / 15 | 500 | Metadatos |

### 3.5 Espaciado y rejilla

Base de **4 px**; escala `4, 8, 12, 16, 24, 32, 48, 64`. Margen lateral de contenido: 32 px (24 px por debajo de 1100 px de ancho). Separación entre tarjetas: 16 px. Separación vertical entre estanterías: 32 px.

### 3.6 Radios (concéntricos)

| Token | Valor | Uso |
|---|---|---|
| `r.xs` | 6 | Chips pequeños, *tooltips* |
| `r.sm` | 10 | Campos, filas |
| `r.md` | 16 | Tarjetas, menús |
| `r.lg` | 22 | Fichas, paneles, portada destacada |
| `r.xl` | 28 | Dock flotante |
| `r.pill` | 999 | Botones, chips, campo de búsqueda |

Regla de concentricidad: radio interno = radio externo − relleno (p. ej. tarjeta de 16 con relleno 6 → imagen interior de 10).

### 3.7 Elevación

| Nivel | Tratamiento (oscuro) |
|---|---|
| 0 | Fondo plano `bg.base` con un degradado radial tenue de `accent` al 8 % en la esquina superior |
| 1 | `bg.surface`, borde `glass.stroke` |
| 2 | `bg.raised`, sombra difusa `0 8 24 negro 35 %` |
| 3 | `bg.overlay`, sombra `0 24 64 negro 50 %` + borde superior de 1 px blanco 14 % (reflejo de luz) |
| Cristal | `glass.fill` + `glass.stroke` + degradado vertical blanco 6 % → 0 % |

> **Limitación declarada**: `egui` no tiene desenfoque de fondo. El «cristal» se simula con capas translúcidas, degradado y reflejo; es una aproximación, no un *backdrop blur* real (research.md §2).

### 3.8 Movimiento

| Token | Valor | Uso |
|---|---|---|
| `dur.fast` | 120 ms | *Hover*, pulsación |
| `dur.base` | 200 ms | Cambios de estado, chips |
| `dur.slow` | 320 ms | Fichas, cambio de pantalla |
| `spring.snappy` | respuesta 0,30 s, amortiguación 0,85 | Pulsación, selección |
| `spring.smooth` | respuesta 0,45 s, amortiguación 0,90 | Fichas, estanterías |
| `ease.out` | cúbica `(0.2, 0, 0, 1)` | Entradas |

- Los muelles se implementan en `motion.rs` como integración semi-implícita por fotograma (`dt` acotado), sin dependencia externa.
- **Reducir movimiento**: `dur.* = 0`, sin muelles ni *parallax*; los cambios son instantáneos o con fundido de 80 ms. Se activa con el ajuste del sistema o `reduce_motion = on`.
- Todo movimiento es **interrumpible**: un clic durante una transición recoge el valor actual y continúa hacia el nuevo objetivo.

## 4. Componentes

### 4.1 Dock lateral flotante (Meta Quest)
Cápsula vertical de `r.xl`, 72 px de ancho, separada 16 px del borde, con iconos de 24 px (Phosphor) y etiqueta en *tooltip*. El elemento activo lleva un fondo `accent.gradient` al 22 % y el icono en `accent`. Entradas: Inicio, Buscar, Explorar, Biblioteca, Descargas; abajo: Ajustes y el indicador de red. En anchos < 960 px no se colapsa (el mínimo de ventana lo impide).

### 4.2 Portada destacada (hero)
Altura 320 px, `r.lg`, portada generada a todo ancho con degradado inferior hacia `bg.base` para el texto. Contenido: etiqueta de sección, título `display`, línea de metadatos, y dos botones (primario *Ver archivos*, secundario *Añadir a favoritos*). Rota entre 5 títulos de «Recientes» cada 8 s (pausable; no rota con *reducir movimiento*).

### 4.3 Estantería (shelf)
Cabecera `title2` + enlace «Ver todo». Fila horizontal desplazable (rueda del ratón con *shift*, arrastre y flechas); tarjetas de 160×240 (2:3) con 16 px de separación y *snap* suave. Las flechas laterales aparecen al pasar el ratón.

### 4.4 Tarjeta de título
Portada generada `r.md`; debajo `headline` (máx. 2 líneas, título limpio) y `caption` (año · tipo). *Hover*: elevación 2, escala 1,03 con `spring.snappy`, y botones rápidos (favorito, descargar) sobre la portada. Foco de teclado: anillo `focus.ring`.

### 4.5 Ficha de título (sheet)
Panel de `r.lg` a la derecha (ancho 520 px) o centrado en ventanas estrechas, con fondo `bg.overlay` y reflejo superior. Cabecera con portada ampliada, `title1`, metadatos y acciones; debajo, lista de archivos (nombre, tamaño, fecha, botón de descarga). Se cierra con `Esc` o clic fuera.

### 4.6 Botones

| Tipo | Aspecto |
|---|---|
| Primario | Cápsula, relleno `accent.gradient`, texto blanco `callout` 600, altura 40, relleno horizontal 20; *hover* `accent.hover`; pulsado escala 0,97 |
| Secundario | Cápsula, `glass.fill` + `glass.stroke`, texto `text.primary` |
| Terciario | Solo texto `accent` |
| Icono | Círculo 36 px, `glass.fill` al pasar el ratón |
| Destructivo | Como secundario con texto/icono `danger` |

### 4.7 Campo de búsqueda
Cápsula grande (altura 48) con icono, texto `body`, indicador `Ctrl K` y botón de limpiar. Foco: borde `accent` de 2 px. Debajo, chips de filtro (Todo/Archivos/Carpetas · Vídeo/Subtítulos · año).

### 4.8 Chip
Cápsula 28 px, `callout`; activo: relleno `accent` al 20 % y texto `accent`.

### 4.9 Anillo de progreso
Círculo de 56 px con arco `accent.gradient` de 5 px sobre pista `glass.stroke`; porcentaje al centro; al completar se transforma en ✓ `success` con `spring.snappy`. Estado `paused` con icono de pausa; `error` con `danger`.

### 4.10 Barra de estado
Pie de 28 px, `caption`: `●` verde/ámbar/rojo + «HTTP» o «FTP» + proxy enmascarado (si hay) + estado del índice («29.994 carpetas»). Pulsarla abre Ajustes → Red.

### 4.11 Toast
Cápsula en la esquina inferior derecha, cristal, icono de nivel, 4 s (errores permanecen); máximo 3 apilados.

### 4.12 Estados vacíos
Ilustración simple (forma geométrica del sistema + icono), título `title2`, texto `body` y una acción. Nunca una pantalla en blanco.

### 4.13 Veredicto de diagnóstico
Lista de 4 pasos con icono ✓/✕ (color + forma), milisegundos en `caption`, y debajo un cuadro de veredicto con icono grande, texto en lenguaje llano y la acción recomendada («Usar HTTP»). Botón «Copiar diagnóstico».

## 5. Portadas generadas

Algoritmo (`cover.rs`):

1. `seed = fnv1a(normalize(nombre))`.
2. Paleta: `PALETTES[seed % N]` con **12 paletas curadas** de tres tonos (p. ej. índigo→azul→cian, magenta→violeta→azul, ámbar→naranja→rojo vino…), cada una validada para que el texto blanco tenga ≥ 4,5 : 1 sobre su tono más claro con una capa oscura del 25 %.
3. Degradado lineal con ángulo `45° + (seed >> 8) % 90°`.
4. Una forma suave determinista (círculo, arco o par de ondas) con blanco al 10–16 %, posición y tamaño derivados de `seed`.
5. Texto: iniciales o primera palabra del título en `display` con blanco al 92 %; abajo, sección y año en `caption`.
6. Icono de tipo en una esquina: película, serie, música, libro, documental, vídeo, carpeta.

Propiedades: **determinista** (mismo nombre → misma portada), **sin red**, **instantánea** (se dibuja con el *painter*; se cachea como textura por `seed` y tamaño).

Clasificación por tipo (para el icono) según `section`: `Peliculas`/`Infantiles` → película; `Series`/`Reality Shows` → serie; `MP3s` → música; `Libros` → libro; `Documentales` → documental; otras → carpeta.

## 6. Pantallas (resumen visual)

Detalle de estados y flujos en [contracts/ui-screens.md](contracts/ui-screens.md).

| Pantalla | Composición |
|---|---|
| Inicio | Dock · portada destacada (rota entre las novedades) · *Continuar descargando* · *Novedades* (carpeta `/Recientes/` del sitio, por fecha) · *Tus favoritos* · *Categorías* (una ficha por sección raíz). Las secciones del sitio son categorías, no títulos (research §14), por eso van como fichas y no como estanterías de portadas |
| Buscar | Campo grande centrado al inicio → se desplaza arriba al escribir; resultados en rejilla de tarjetas (carpetas) y filas (archivos) |
| Explorar | Migas de pan en cápsula + rejilla de carpetas + lista de archivos; botón *Descargar carpeta* |
| Ficha | Panel `r.lg` con portada ampliada y lista de archivos |
| Biblioteca | Pestañas en cápsula: Favoritos · Listas · Para ver |
| Descargas | Filas con anillo, nombre, velocidad, restante y acciones; cabecera con totales |
| Ajustes | Secciones: Red (modo, proxy, diagnóstico) · Descargas · Apariencia · Reproductor · Caché · Acerca de |

## 7. Accesibilidad

| Requisito | Implementación |
|---|---|
| Teclado | Todo es alcanzable con `Tab`/flechas; `Enter` activa; `Esc` cierra; atajos de FR-043 |
| Foco visible | `focus.ring` de 2 px en cualquier elemento enfocable |
| Contraste | Tokens validados (§3.3) |
| Zoom | `ui_scale` 0,8–2,0 y `Ctrl +/−/0`; el diseño no depende de píxeles absolutos |
| Reducir movimiento | §3.8 |
| No solo color | Estados llevan icono y texto (✓/✕, «Pausada», «Error») |
| Lectores de pantalla | `egui` expone AccessKit; las acciones y tarjetas llevan etiquetas |
| Objetivos táctiles/clic | Mínimo 32×32 px, 40 px en botones primarios |

## 8. Tema y respeto al sistema

`theme = system` sigue el modo claro/oscuro del sistema; por defecto **oscuro** (los catálogos de vídeo se consumen mejor en oscuro). `reduce_motion = system` consulta la preferencia del sistema donde exista; si no, queda activo el movimiento.

## 9. Lo que no se hace

- No se usa el rojo de Netflix, el azul exacto de Facebook (`#1877F2`) ni tipografías propietarias de Apple/Meta.
- No hay *skeuomorfismo*, sombras duras ni bordes gruesos.
- No hay animaciones decorativas en bucle (salvo el anillo de progreso activo y el rotador de la portada, ambos desactivables).
- No hay texto sobre portada sin la capa oscura que garantiza contraste.

## 10. Hallazgos de verificación

- **2026-11-14 · contraste del botón primario**: la prueba `theme::tests::dark_palette_meets_wcag_aa` falló con el diseño inicial: texto blanco sobre `#0A84FF` da **3,65 : 1** (< 4,5). Se introdujo `accent.fill` (`#0A70E0`, 4,77 : 1) para el relleno de botones y se reservó `#0A84FF` para texto, iconos y foco sobre fondo oscuro (donde sí supera 3 : 1). Un botón no cambia de color en *hover* (aclarar el relleno rompería el contraste): se resalta con un halo de `accent` de 2 px y la escala del §4.6.
