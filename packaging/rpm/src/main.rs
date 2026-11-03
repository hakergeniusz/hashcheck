// SPDX-License-Identifier: MIT
//! Builds the Hashcheck .rpm without rpmbuild.
//!
//! Usage (from the repository root): cargo run --release --manifest-path packaging/rpm/Cargo.toml -- <repo-root> <out-dir>

use std::path::{Path, PathBuf};

const VERSION: &str = "1.0.0";

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err("usage: build-rpm <repo-root> <out-dir>".into());
    }
    let repo = PathBuf::from(&args[1]);
    let out_dir = Path::new(&args[2]);
    std::fs::create_dir_all(out_dir)?;

    let description = "Hashcheck lets you hash a file, verify it against a known hash \
and compare two files.\n\nSupported hash functions: MD5, SHA-1, SHA-256, SHA-512, \
Blake3, CRC32 and Adler32.\n\nRust + GTK4/libadwaita rewrite of Collision.";

    let mut builder = rpm::PackageBuilder::new(
        "hashcheck",
        VERSION,
        "MIT",
        "x86_64",
        "Check hashes for your files",
    );
    builder
        .description(description)
        .url("https://github.com/hakergeniusz/hashcheck")
        .vendor("hakergeniusz")
        .packager("hakergeniusz")
        .requires(rpm::Dependency::greater_eq("gtk4", "4.14"))
        .requires(rpm::Dependency::greater_eq("libadwaita", "1.9"));

    builder.with_file(
        repo.join("target/release/hashcheck"),
        rpm::FileOptions::new("/usr/bin/hashcheck").permissions(0o755),
    )?;
    builder.with_file(
        repo.join("data/io.github.hakergeniusz.Hashcheck.desktop"),
        rpm::FileOptions::new("/usr/share/applications/io.github.hakergeniusz.Hashcheck.desktop")
            .permissions(0o644),
    )?;
    builder.with_file(
        repo.join("data/io.github.hakergeniusz.Hashcheck.metainfo.xml"),
        rpm::FileOptions::new("/usr/share/metainfo/io.github.hakergeniusz.Hashcheck.metainfo.xml")
            .permissions(0o644),
    )?;
    builder.with_file(
        repo.join("data/io.github.hakergeniusz.Hashcheck.gschema.xml"),
        rpm::FileOptions::new(
            "/usr/share/glib-2.0/schemas/io.github.hakergeniusz.Hashcheck.gschema.xml",
        )
        .permissions(0o644),
    )?;
    builder.with_file(
        repo.join("data/icons/io.github.hakergeniusz.Hashcheck.svg"),
        rpm::FileOptions::new(
            "/usr/share/icons/hicolor/scalable/apps/io.github.hakergeniusz.Hashcheck.svg",
        )
        .permissions(0o644),
    )?;
    builder.with_file(
        repo.join("data/icons/io.github.hakergeniusz.Hashcheck-symbolic.svg"),
        rpm::FileOptions::new(
            "/usr/share/icons/hicolor/symbolic/apps/io.github.hakergeniusz.Hashcheck-symbolic.svg",
        )
        .permissions(0o644),
    )?;
    builder.with_file(
        repo.join("LICENSE"),
        rpm::FileOptions::new("/usr/share/licenses/hashcheck/LICENSE").permissions(0o644),
    )?;

    for icon in [
        "hashcheck-hash-symbolic",
        "hashcheck-file-symbolic",
        "hashcheck-success-symbolic",
        "hashcheck-error-symbolic",
    ] {
        builder.with_file(
            repo.join(format!("data/icons/{icon}.svg")),
            rpm::FileOptions::new(format!("/usr/share/icons/hicolor/symbolic/actions/{icon}.svg"))
                .permissions(0o644),
        )?;
    }

    let package = builder.build()?;
    let path = package.write_to(out_dir)?;
    println!("wrote {}", path.display());
    Ok(())
}
