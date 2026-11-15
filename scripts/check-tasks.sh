#!/usr/bin/env bash
# Constitución V: ninguna tarea se marca hecha ([x]) sin evidencia en specs/*/tasks.md.
set -euo pipefail
cd "$(dirname "$0")/.."
bad=$(grep -hnE '^- \[x\] \*\*T-' specs/*/tasks.md | grep -v 'Evidencia' || true)
if [ -n "$bad" ]; then
  echo "Tareas marcadas [x] sin «Evidencia»:" >&2
  echo "$bad" >&2
  exit 1
fi
echo "tasks.md: todas las tareas hechas tienen evidencia."
