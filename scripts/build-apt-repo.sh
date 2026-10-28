#!/usr/bin/env bash
# Genera y publica el repositorio APT en la rama gh-pages (uso local).
# Requiere: dpkg-dev, apt-utils, gnupg, gh autenticado.
set -euo pipefail

TAG="${1:-v0.1.0}"
REPO="novatic-cu/NovaFlix"
OUT="$(mktemp -d)"
trap 'rm -rf "$OUT"' EXIT

echo "==> Descargando .deb de $TAG"
gh release download "$TAG" --repo "$REPO" --pattern "*.deb" --dir "$OUT/debs"

mkdir -p "$OUT/public/apt/pool/main" "$OUT/public/apt/dists/stable"
cd "$OUT/public/apt"

for deb in "$OUT"/debs/*.deb; do
  pkg=$(dpkg-deb -f "$deb" Package)
  ver=$(dpkg-deb -f "$deb" Version)
  arch=$(dpkg-deb -f "$deb" Architecture)
  prefix=$(echo "$pkg" | cut -c1)
  mkdir -p "pool/main/${prefix}/${pkg}"
  cp "$deb" "pool/main/${prefix}/${pkg}/${pkg}_${ver}_${arch}.deb"
done

dpkg-scanpackages --multiversion pool /dev/null > dists/stable/Packages
gzip -9c dists/stable/Packages > dists/stable/Packages.gz

cat > dists/stable/Release <<EOF
Origin: NovaFlix
Label: NovaFlix
Suite: stable
Codename: stable
Architectures: amd64 arm64
Components: main
Description: Buscador de escritorio para catalogos multimedia publicos
Date: $(date -Ru)
EOF
apt-ftparchive release dists/stable >> dists/stable/Release

if [[ -n "${APT_GPG_KEY_ID:-}" ]]; then
  gpg --batch --yes --local-user "$APT_GPG_KEY_ID" --clearsign -o dists/stable/InRelease dists/stable/Release
  gpg --batch --yes --local-user "$APT_GPG_KEY_ID" --armor --detach-sign -o dists/stable/Release.gpg dists/stable/Release
  gpg --armor --export "$APT_GPG_KEY_ID" > novaflix.gpg
  echo "==> Repositorio firmado con $APT_GPG_KEY_ID"
else
  echo "==> Sin APT_GPG_KEY_ID: repositorio sin firmar (usa trusted=yes)"
fi

echo "==> Listo en $OUT/public/apt"
echo "    Publícalo copiando public/ a la rama gh-pages o deja que el workflow apt.yml lo haga."
