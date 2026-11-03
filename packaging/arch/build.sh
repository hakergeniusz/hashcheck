#!/bin/bash
# SPDX-License-Identifier: MIT
# Builds the Arch package from the repository tree: ./build.sh [output-dir]
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-"$repo/dist"}
mkdir -p "$out"
out=$(cd "$out" && pwd)
pkgver=$(grep -m1 '^version' "$repo/Cargo.toml" | cut -d'"' -f2)
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT

mkdir "$stage/hashcheck-$pkgver"
cp -r "$repo"/{Cargo.toml,Cargo.lock,build.rs,src,data,LICENSE} "$stage/hashcheck-$pkgver/"
tar -czf "$stage/hashcheck-$pkgver.tar.gz" -C "$stage" "hashcheck-$pkgver"
cp "$repo/packaging/arch/PKGBUILD" "$stage/"

cd "$stage"
# Reuse the repository's cargo target dir when one was exported.
makepkg -f --noconfirm

mv hashcheck-*.pkg.tar.zst "$out/"
echo "wrote $out"/hashcheck-*.pkg.tar.zst
