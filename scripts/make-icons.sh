#!/usr/bin/env bash
# Genera todos los derivados del icono a partir de los SVG de assets/ (idempotente).
# Requiere: rsvg-convert (librsvg) y magick (ImageMagick). Los derivados se versionan.
set -euo pipefail
cd "$(dirname "$0")/.."

MASTER=assets/logo.svg
SMALL=assets/logo-small.svg

png() { # png <svg> <tamaño> <salida>
  rsvg-convert -w "$2" -h "$2" "$1" -o "$3"
}
pick() { [ "$1" -le 48 ] && echo "$SMALL" || echo "$MASTER"; }

# Linux (hicolor) y AUR/tarball
for n in 16 22 24 32 48 64 128 256 512; do
  mkdir -p "packaging/icons/hicolor/${n}x${n}/apps"
  png "$(pick $n)" "$n" "packaging/icons/hicolor/${n}x${n}/apps/novaflix.png"
done
mkdir -p packaging/aur/icons
for n in 32 128 256 512; do
  cp "packaging/icons/hicolor/${n}x${n}/apps/novaflix.png" "packaging/aur/icons/novaflix-${n}.png"
done

# Activos del repositorio (README, sitio)
for n in 32 64 128 256 512; do
  png "$(pick $n)" "$n" "assets/icon-${n}.png"
done
png "$MASTER" 256 assets/logo-256.png

# Aplicación: icono de ventana y recurso de Windows
APP=crates/novaflix-app/assets/icons
mkdir -p "$APP"
png "$MASTER" 256 "$APP/icon.png"
TMP=$(mktemp -d); trap 'rm -rf "$TMP"' EXIT
for n in 16 20 24 32 40 48 64 128 256; do png "$(pick $n)" "$n" "$TMP/$n.png"; done
magick "$TMP"/16.png "$TMP"/20.png "$TMP"/24.png "$TMP"/32.png "$TMP"/40.png \
       "$TMP"/48.png "$TMP"/64.png "$TMP"/128.png "$TMP"/256.png "$APP/icon.ico"

# Vista previa para redes (1280×640); el SVG referencia assets/logo.svg
rsvg-convert -w 1280 -h 640 assets/social-preview.svg -o assets/social-preview.png
echo "Iconos generados."
