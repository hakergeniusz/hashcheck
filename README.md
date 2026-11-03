# Hashcheck

<p align="center">
  <img alt="branding" width="192" src="./data/icons/io.github.hakergeniusz.Hashcheck.svg">
</p>
<h1 align="center">Hashcheck</h1>
<h4 align="center">Check hashes for your files</h4>

Hashcheck lets you check hashes for your files: hash a file, verify it against
a known hash or compare two files.

Supported hash functions: **MD5**, **SHA-1**, **SHA-256**, **SHA-512**,
**Blake3**, **CRC32** and **Adler32**.

It is a Rust rewrite of
[Collision](https://codeberg.org/GeopJr/Collision) (originally written in
Crystal), built with GTK4 + libadwaita via [gtk4-rs](https://github.com/gtk-rs/gtk4-rs).

## Features

- Drag & drop or open a file to calculate all hashes
- Verify a pasted hash against the calculated ones
- Compare two files directly
- Toggle which hash functions get calculated
- Copy hashes individually or as a Markdown table
- Remembers window size and enabled hash functions (GSettings)
- Multiple windows, Ctrl+N / Ctrl+O / Ctrl+W / Ctrl+Q / F1 shortcuts

## Building

Dependencies: Rust (1.85+), GTK4 (4.14+), libadwaita (1.9+).

```sh
make          # cargo build --release --locked
make install  # to /usr (add DESTDIR/PREFIX as needed)
```

To run from the source tree (no schema installed, settings fall back to
in-memory defaults):

```sh
cargo run
```

## Flatpak

```sh
flatpak install org.gnome.Platform//51 org.gnome.Sdk//51
flatpak-builder --user --install --force-clean build-dir data/io.github.hakergeniusz.Hashcheck.json
```

## Packaging

- Debian: `python3 packaging/deb/build-deb.py . --out dist/hashcheck_1.0.0_amd64.deb`
- Fedora: `cargo run --release --manifest-path packaging/rpm/Cargo.toml -- . dist/`
- Arch: `packaging/arch/build.sh` (stages a source tarball, runs makepkg, writes to `dist/`)
- Flatpak bundle: build with flatpak-builder, then `flatpak build-bundle ~/.local/share/flatpak/repo hashcheck.flatpak io.github.hakergeniusz.Hashcheck --runtime-repo=https://flathub.org/repo/flathub.flatpakrepo`


## License

Hashcheck is licensed under the MIT license — see [LICENSE](./LICENSE).

It is a rewrite of Collision by Evangelos "GeopJr" Paterakis (BSD-2-Clause),
whose BSD-2 copyright notice is included in the MIT license text above.
This project is not affiliated with or endorsed by the original author.
