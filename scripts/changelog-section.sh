#!/usr/bin/env bash
# Imprime la sección de una versión de CHANGELOG.md (para el cuerpo del release).
# Uso: scripts/changelog-section.sh 2.0.0-alpha.1
set -euo pipefail
cd "$(dirname "$0")/.."
awk -v v="$1" '
  $0 ~ "^## \\[" v "\\]" { found=1; next }
  found && /^## \[/ { exit }
  found { print }
' CHANGELOG.md
