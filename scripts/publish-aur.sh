#!/usr/bin/env bash
# Publica novaflix-bin en el AUR.
# Requisitos: cuenta AUR con tu clave SSH registrada, git, makepkg.
# Uso: ./scripts/publish-aur.sh 0.1.0
set -euo pipefail

VERSION="${1:?uso: publish-aur.sh <version>}"
PKG="novaflix-bin"
ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

cd "$ROOT"

# Sincroniza versión y hash del tarball publicado.
URL="https://github.com/novatic-cu/NovaFlix/releases/download/v${VERSION}/novaflix-${VERSION}-x86_64.tar.gz"
SHA=$(curl -fsSL "$URL" | sha256sum | cut -d' ' -f1)
sed -i "s/^pkgver=.*/pkgver=${VERSION}/" packaging/aur/PKGBUILD
sed -i "s/^sha256sums=.*/sha256sums=('${SHA}')/" packaging/aur/PKGBUILD

( cd packaging/aur && makepkg --printsrcinfo > .SRCINFO )

echo "==> Clonando AUR"
git clone "ssh://aur@aur.archlinux.org/${PKG}.git" "$WORK/$PKG"
cp packaging/aur/PKGBUILD packaging/aur/.SRCINFO "$WORK/$PKG/"
cd "$WORK/$PKG"

git add PKGBUILD .SRCINFO
git commit -m "upgpkg: ${PKG} ${VERSION}-1"
git push origin master
echo "==> ${PKG} ${VERSION}-1 publicado en AUR"
