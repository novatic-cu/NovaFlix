#!/usr/bin/env bash
# Crea novaflix-<ver>-x86_64.tar.gz (binario + .desktop + iconos): lo consumen el AUR y el paquete Arch.
# Uso: scripts/make-tarball.sh <versión> <ruta-del-binario> <directorio-de-salida>
set -euo pipefail
cd "$(dirname "$0")/.."
VERSION="${1:?versión}"; BIN="${2:?binario}"; OUT="${3:?salida}"
STAGE=$(mktemp -d); trap 'rm -rf "$STAGE"' EXIT
install -Dm755 "$BIN" "$STAGE/novaflix"
install -Dm644 packaging/novaflix.desktop "$STAGE/novaflix.desktop"
for n in 16 22 24 32 48 64 128 256 512; do
  install -Dm644 "packaging/icons/hicolor/${n}x${n}/apps/novaflix.png" "$STAGE/icons/novaflix-${n}.png"
done
mkdir -p "$OUT"
tar -C "$STAGE" -czf "$OUT/novaflix-${VERSION}-x86_64.tar.gz" novaflix novaflix.desktop icons
echo "$OUT/novaflix-${VERSION}-x86_64.tar.gz"
