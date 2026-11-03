// SPDX-License-Identifier: MIT
//! Compiles the bundled action icons into a GResource embedded in the binary.

use std::path::PathBuf;
use std::process::Command;

fn main() {
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR not set"));
    let target = out_dir.join("hashcheck.gresource");

    println!("cargo:rerun-if-changed=data/hashcheck.gresource.xml");
    println!("cargo:rerun-if-changed=data/icons");

    let status = Command::new("glib-compile-resources")
        .args([
            "--sourcedir=data",
            "data/hashcheck.gresource.xml",
            "--target",
        ])
        .arg(&target)
        .status()
        .expect("glib-compile-resources not found (install glib2)");
    assert!(status.success(), "glib-compile-resources failed");
}
