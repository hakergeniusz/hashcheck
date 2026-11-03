#!/bin/bash
# SPDX-License-Identifier: MIT
# Builds the portable tarball from the repository tree: ./build.sh [output-dir]
# Expects a release binary at target/release/hashcheck (make) unless one is provided.
set -euo pipefail

repo=$(cd "$(dirname "$0")/../.." && pwd)
out=${1:-"$repo/dist"}
mkdir -p "$out"
out=$(cd "$out" && pwd)
pkgver=$(grep -m1 '^version' "$repo/Cargo.toml" | cut -d'"' -f2)
binary=${2:-"$repo/target/release/hashcheck"}

[ -x "$binary" ] || { echo "no binary at $binary — run make first" >&2; exit 1; }

stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
mkdir "$stage/hashcheck-$pkgver"
cp "$binary" "$stage/hashcheck-$pkgver/hashcheck"
cp "$repo/README.md" "$repo/LICENSE" "$stage/hashcheck-$pkgver/"
cp "$repo"/data/*.desktop "$repo"/data/*.gschema.xml "$stage/hashcheck-$pkgver/"
mkdir "$stage/hashcheck-$pkgver/icons"
cp "$repo"/data/icons/*.svg "$stage/hashcheck-$pkgver/icons/"

tar -czf "$out/hashcheck-$pkgver-linux-x86_64.tar.gz" -C "$stage" "hashcheck-$pkgver"
echo "wrote $out/hashcheck-$pkgver-linux-x86_64.tar.gz"
