#!/usr/bin/env python3
# SPDX-License-Identifier: MIT
"""Builds a Debian .deb without dpkg-deb (uses ar + tar from binutils).

Usage: build-deb.py <repo-root> [--binary target/release/hashcheck] --out <file.deb>
"""

import argparse
import hashlib
import io
import subprocess
import sys
import tarfile
import tempfile
from pathlib import Path

VERSION = "1.0.0"
MAINTAINER = "hakergeniusz <hakergeniusz@users.noreply.github.com>"

# (source path relative to repo root, target path in the image, mode)
FILES = [
    ("target/release/hashcheck", "/usr/bin/hashcheck", 0o755),
    ("data/io.github.hakergeniusz.Hashcheck.desktop", "/usr/share/applications/io.github.hakergeniusz.Hashcheck.desktop", 0o644),
    ("data/io.github.hakergeniusz.Hashcheck.metainfo.xml", "/usr/share/metainfo/io.github.hakergeniusz.Hashcheck.metainfo.xml", 0o644),
    ("data/io.github.hakergeniusz.Hashcheck.gschema.xml", "/usr/share/glib-2.0/schemas/io.github.hakergeniusz.Hashcheck.gschema.xml", 0o644),
    ("data/icons/io.github.hakergeniusz.Hashcheck.svg", "/usr/share/icons/hicolor/scalable/apps/io.github.hakergeniusz.Hashcheck.svg", 0o644),
    ("data/icons/io.github.hakergeniusz.Hashcheck-symbolic.svg", "/usr/share/icons/hicolor/symbolic/apps/io.github.hakergeniusz.Hashcheck-symbolic.svg", 0o644),
    ("LICENSE", "/usr/share/doc/hashcheck/copyright", 0o644),
]

DIRS = [
    "/usr", "/usr/bin", "/usr/share", "/usr/share/applications",
    "/usr/share/metainfo", "/usr/share/glib-2.0", "/usr/share/glib-2.0/schemas",
    "/usr/share/icons", "/usr/share/icons/hicolor", "/usr/share/icons/hicolor/scalable",
    "/usr/share/icons/hicolor/scalable/apps", "/usr/share/icons/hicolor/symbolic",
    "/usr/share/icons/hicolor/symbolic/apps",
    "/usr/share/doc", "/usr/share/doc/hashcheck",
]

POSTINST = """#!/bin/sh
set -e
if command -v glib-compile-schemas >/dev/null 2>&1; then
    glib-compile-schemas /usr/share/glib-2.0/schemas >/dev/null 2>&1 || true
fi
if command -v update-desktop-database >/dev/null 2>&1; then
    update-desktop-database -q /usr/share/applications >/dev/null 2>&1 || true
fi
"""


def gz_tar(members):
    buf = io.BytesIO()
    with tarfile.open(fileobj=buf, mode="w:gz", format=tarfile.GNU_FORMAT) as tf:
        for name, data, mode in members:
            info = tarfile.TarInfo(name)
            info.size = len(data)
            info.mode = mode
            info.mtime = 0
            info.uid = 0
            info.gid = 0
            info.uname = "root"
            info.gname = "root"
            tf.addfile(info, io.BytesIO(data))
    return buf.getvalue()


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("repo", type=Path)
    ap.add_argument("--binary", type=Path, default=Path("target/release/hashcheck"))
    ap.add_argument("--out", type=Path, required=True)
    args = ap.parse_args()
    repo = args.repo.resolve()
    binary = repo / args.binary
    if not binary.is_file():
        raise SystemExit(f"binary not found: {binary}")

    data_members = []
    md5s = []
    total_kb = 0
    for src, dest, mode in FILES:
        path = binary if src == "target/release/hashcheck" else repo / src
        blob = path.read_bytes()
        data_members.append((f".{dest}", blob, mode))
        md5s.append((dest.lstrip("/"), hashlib.md5(blob).hexdigest()))
        total_kb += max(1, len(blob) // 1024)

    for d in DIRS:
        data_members.append((f".{d}", b"", 0o755))

    control = (
        f"Package: hashcheck\n"
        f"Version: {VERSION}\n"
        f"Section: utils\n"
        f"Priority: optional\n"
        f"Architecture: amd64\n"
        f"Installed-Size: {total_kb}\n"
        f"Maintainer: {MAINTAINER}\n"
        f"Depends: libgtk-4-1 (>= 4.14), libadwaita-1-0 (>= 1.9)\n"
        f"Description: Check hashes for your files\n"
        f" Hashcheck lets you hash a file, verify it against a known hash\n"
        f" and compare two files. Supported hash functions: MD5, SHA-1,\n"
        f" SHA-256, SHA-512, Blake3, CRC32 and Adler32.\n"
        f" .\n"
        f" Rust + GTK4/libadwaita rewrite of Collision.\n"
    )

    control_members = [
        ("./control", control.encode(), 0o644),
        ("./md5sums", ("\n".join(f"{h}  {p}" for p, h in md5s) + "\n").encode(), 0o644),
        ("./postinst", POSTINST.encode(), 0o755),
    ]

    args.out.parent.mkdir(parents=True, exist_ok=True)
    args.out.unlink(missing_ok=True)  # ar rc appends to existing archives
    # .deb is an ar archive: debian-binary, control.tar.gz, data.tar.gz.
    # Assembled with the real `ar` to guarantee a well-formed archive.
    with tempfile.TemporaryDirectory() as td:
        tmp = Path(td)
        (tmp / "debian-binary").write_bytes(b"2.0\n")
        (tmp / "control.tar.gz").write_bytes(gz_tar(control_members))
        (tmp / "data.tar.gz").write_bytes(gz_tar(data_members))
        subprocess.run(
            ["ar", "rc", str(args.out.resolve()), "debian-binary", "control.tar.gz", "data.tar.gz"],
            check=True,
            cwd=td,
        )
    print(f"wrote {args.out} ({args.out.stat().st_size} bytes)")


if __name__ == "__main__":
    main()
