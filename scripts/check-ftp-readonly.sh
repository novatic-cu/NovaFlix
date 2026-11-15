#!/usr/bin/env bash
# Constitución IV: el cliente FTP no puede emitir comandos de escritura.
set -euo pipefail
cd "$(dirname "$0")/.."
file=crates/novaflix-core/src/transport/ftp.rs
# Se ignoran los tests (a partir de `mod tests`) y los comentarios.
body=$(sed '/^#\[cfg(test)\]/,$d' "$file" | grep -vE '^\s*//')
if echo "$body" | grep -nE '"(STOR|APPE|DELE|RMD|MKD|RNFR|RNTO|SITE|STOU)\b|\b(STOR|APPE|DELE|RMD|MKD|RNFR|RNTO|STOU) ' ; then
  echo "ftp.rs contiene comandos FTP de escritura" >&2
  exit 1
fi
echo "ftp.rs: solo comandos de lectura."
