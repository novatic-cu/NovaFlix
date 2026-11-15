# Contrato: pantallas, estados y flujos de la interfaz

**Spec**: [../spec.md](../spec.md) · **Diseño**: [../design-system.md](../design-system.md) · **Núcleo**: [core-api.md](core-api.md)

Cada pantalla define: propósito, fuente de datos (API del núcleo), estados (Cargando / Vacío / Error / Normal), interacciones y atajos. Los estados son **obligatorios** (Principio III y diseño §2.5).

## 0. Estructura de ventana

| Zona | Contenido |
|---|---|
| Izquierda | Dock flotante (Inicio, Buscar, Explorar, Biblioteca, Descargas · Ajustes) |
| Centro | Pantalla activa |
| Derecha (superpuesta) | Ficha de título (cuando hay una abierta) |
| Pie | Barra de estado (protocolo, proxy, índice) |
| Esquina inferior derecha | Toasts |

Ventana: 1180×780 por defecto, mínimo 960×640, centrada. Una sola instancia (la segunda enfoca la primera).

## 1. Arranque

1. `Core::open` (migración si procede).
2. Si hay `migration_notice`, toast de aviso con la ruta de la copia.
3. `bootstrap(false)`: si el índice es fresco, `IndexReady` inmediato; si no, estado «Preparando el catálogo…».
4. `run_probe()` en segundo plano (actualiza la barra de estado).
5. Si es la primera ejecución (sin ajustes de red guardados): tarjeta de bienvenida de una pantalla con el resultado del diagnóstico y «Empezar».

| Estado | Pantalla |
|---|---|
| Cargando índice (primera vez, sin caché) | Portada con progreso indeterminado y texto «Preparando el catálogo (solo la primera vez)…» |
| Error de red en el primer bootstrap | Estado de error con veredicto del diagnóstico y botones *Reintentar* · *Ajustes de red* |

## 2. Inicio

**Datos**: `browse("/Recientes/")`, `sections()`, `favorites()`, `downloads()` (activas).

| Bloque | Origen | Vacío |
|---|---|---|
| Portada destacada | Las 5 carpetas más nuevas de `/Recientes/` (rota cada 8 s; no rota con «reducir movimiento») | Esqueleto mientras carga |
| Continuar descargando | `downloads()` con estado `active/paused/queued` | Se oculta |
| Novedades | Listado de `/Recientes/` (carpetas y archivos) ordenado por fecha descendente | «No se pudieron cargar las novedades» + *Reintentar* |
| Tus favoritos | `favorites()` | Se oculta |
| Categorías | `sections()` (secciones raíz del índice) con su número de carpetas | Se oculta hasta que haya índice |

**Interacciones**: clic en tarjeta → ficha; `Enter` en foco → ficha; «Ver todo» → Explorar en esa sección.

## 3. Buscar

**Datos**: `search(q, filter, 60, offset)` (síncrono, con *debounce* de 80 ms), `recent_searches()`.

| Estado | Pantalla |
|---|---|
| Inicial (vacío) | Campo centrado grande, debajo «Búsquedas recientes» (chips) y atajos |
| Cargando | No aplica (síncrono < 50 ms); si tarda > 150 ms, línea de progreso fina |
| Sin resultados | «No encontramos «x»» + sugerencia («Prueba con menos palabras o sin año») + secciones |
| Con resultados | Contador y tiempo («4 resultados · 7 ms»); carpetas en rejilla de tarjetas, archivos en filas |
| Error | Texto del error + *Reintentar* (p. ej. índice no listo → botón *Preparar catálogo*) |

**Filtros**: Todo/Archivos/Carpetas · Cualquiera/Vídeo/Subtítulos · Año · Sección.
**Atajos**: `Ctrl+K` enfoca; `↓` baja a resultados; `Esc` limpia.
**Paginación**: *scroll* infinito pidiendo la siguiente página (`offset += 60`).

## 4. Explorar

**Datos**: `browse(path, force)` (asíncrono) + `browse_cached(path)` para pintar al instante.

| Estado | Pantalla |
|---|---|
| Cargando | Esqueletos de tarjetas (sin movimiento con *reducir movimiento*) |
| Con datos | Migas de pan (`Peliculas › 2024`), rejilla de carpetas, lista de archivos, botón *Descargar carpeta* |
| Carpeta vacía | «Esta carpeta está vacía» + *Subir un nivel* |
| Error | Texto según `ErrorKind` + acción (Reintentar · Abrir diagnóstico · Cambiar a HTTP) |
| Dato en caché | Insignia «Guardado hace 3 h» + botón *Actualizar* |

**Interacciones**: clic en carpeta → entra; clic en archivo → ficha del archivo; `Retroceso` sube un nivel; `Ctrl+L` edita la ruta.

## 5. Ficha de título

Se abre sobre cualquier pantalla (`Esc` la cierra). Para una **carpeta**: lista sus archivos con `browse`; para un **archivo**: muestra sus metadatos y acciones.

| Elemento | Contenido |
|---|---|
| Cabecera | Portada ampliada, título limpio (`norm`/nombre sin etiquetas de release), año, sección, tamaño total si se conoce |
| Acciones | **Descargar** (primario) · Reproducir · ☆ Favorito · ＋ Lista · Copiar enlace |
| Archivos | Filas: icono por extensión, nombre, tamaño, fecha, botón de descarga individual, contador de descargas (`counters`) |
| Subcarpetas | Tarjetas pequeñas que navegan dentro de la ficha |

| Estado | Comportamiento |
|---|---|
| Listando | Esqueleto de filas |
| Sin archivos de vídeo | Aviso «No hay vídeos aquí; mira las subcarpetas» |
| Reproducir sin reproductor | Diálogo: «No encontramos VLC ni mpv» + *Copiar enlace* · *Elegir reproductor* |
| Descargar | Toast «Añadida a la cola» + atajo a Descargas |

**Descargar carpeta**: abre un diálogo con «N archivos · X GB» (de `FolderPlan`), selector de destino y política de existentes; *Empezar* encola.

## 6. Biblioteca

Pestañas en cápsula.

| Pestaña | Datos | Vacía |
|---|---|---|
| Favoritos | `favorites()` agrupados: carpetas (tarjetas) y archivos (filas), con etiqueta y nota editables | «Marca ☆ en cualquier título para guardarlo aquí» |
| Listas | `lists()` → detalle `list_items()` con reordenado por arrastre, exportar M3U | «Crea tu primera lista» |
| Para ver | Lista de tipo *watchlist* con casilla *visto* | «Nada pendiente» |

## 7. Descargas

**Datos**: `downloads()` + eventos `Download`.

| Elemento | Detalle |
|---|---|
| Cabecera | «2 activas · 1 en cola · 3.2 MB/s» + *Limpiar terminadas* |
| Fila | Anillo de progreso, nombre, `X de Y · velocidad · tiempo restante`, protocolo (`HTTP`/`FTP`), acciones (pausar/reanudar, cancelar, abrir carpeta, reintentar) |
| Vacío | «Aún no has descargado nada» + *Explorar el catálogo* |
| Error en fila | Texto accionable + *Reintentar* |

## 8. Ajustes

| Sección | Controles |
|---|---|
| **Red** | Protocolo (segmentado: Automático · HTTP · FTP); descripción de cada uno; Proxy (Sistema · Directo · Personalizado) con URL, usuario y contraseña (llavero); usuario FTP; **Diagnosticar** con veredicto (design §4.13); *Copiar diagnóstico* |
| **Descargas** | Carpeta; descargas simultáneas (1–3); límite de velocidad |
| **Apariencia** | Tema (Sistema · Oscuro · Claro); escala; reducir movimiento |
| **Reproductor** | Automático / comando; *Probar* |
| **Caché e índice** | TTL, tamaño, modo (ligero/profundo) con *Rastrear ahora*/*Detener*; *Vaciar caché*; estadísticas |
| **Acerca de** | Versión, licencia, enlaces, carpeta de datos, *Abrir carpeta de datos* |

**Validación**: cada campo se valida al salir; errores en línea con texto específico («El proxy debe empezar por http://, https:// o socks5://»). Los cambios de red se aplican con **Aplicar** y relanzan el diagnóstico.

### 8.1 Flujo «mi FTP no funciona»

1. Usuario elige *FTP*.
2. Si `probe` indica FTP inaccesible, aparece bajo el selector el cuadro de veredicto: «Tu proxy no deja pasar FTP (403 …)».
3. Botones: **Usar HTTP** (primario) · *Dejarlo en FTP de todos modos*.
4. Si se deja en FTP y falla una operación: error con *Cambiar a HTTP*.

## 9. Atajos globales

| Atajo | Acción |
|---|---|
| `Ctrl+K` | Buscar |
| `Ctrl+L` | Ir a ruta (Explorar) |
| `Ctrl+1…5` | Inicio, Buscar, Explorar, Biblioteca, Descargas |
| `Ctrl+,` | Ajustes |
| `Ctrl +` / `−` / `0` | Zoom |
| `Esc` | Cerrar ficha / diálogo / limpiar búsqueda |
| `F5` / `Ctrl+R` | Actualizar la carpeta actual |
| `Retroceso` / `Alt+←` | Subir un nivel / atrás |

## 10. Mensajes de error (texto de usuario)

| `ErrorKind` | Título | Texto | Acción |
|---|---|---|---|
| `Offline` | Sin conexión con el servidor | «No se pudo llegar a visuales.uclv.cu. Revisa tu red o tu proxy.» | Diagnosticar |
| `ProxyAuth` | El proxy rechazó las credenciales | «Comprueba el usuario y la contraseña del proxy en Ajustes → Red.» | Ir a Ajustes |
| `ProxyBlocked` | Tu proxy no deja pasar FTP | «NovaFlix usará HTTP, que sí funciona.» | Usar HTTP |
| `FtpLogin` | El servidor FTP pidió credenciales | «Introduce usuario y contraseña en Ajustes → Red.» | Ir a Ajustes |
| `NotFound` | Ya no existe | «Esa carpeta o archivo ya no está en el servidor.» | Actualizar |
| `Disk` | No se pudo escribir en disco | «Comprueba el espacio libre y los permisos de <ruta>.» | Abrir carpeta |
| `Other` | Algo salió mal | Texto del error | Reintentar |
