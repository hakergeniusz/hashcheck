// SPDX-License-Identifier: MIT
//! Diagnostic: which files do icon-theme lookups actually resolve to, and
//! what do the icons look like when drawn?
//! Run under a display: xvfb-run cargo run --release --example icon-probe

use gtk::prelude::*;

fn main() {
    gtk::init().expect("gtk init");
    let display = gtk::gdk::Display::default().expect("display");

    // Same registration order as main.rs: resource first, then theme path.
    let bytes = include_bytes!(concat!(env!("OUT_DIR"), "/hashcheck.gresource"));
    let resource = gtk::gio::Resource::from_data(&gtk::glib::Bytes::from_static(bytes))
        .expect("failed to load bundled icons");
    gtk::gio::resources_register(&resource);

    let theme = gtk::IconTheme::for_display(&display);
    theme.add_resource_path("/io/github/hakergeniusz/Hashcheck/icons");

    for base in [
        "/io/github/hakergeniusz/Hashcheck/icons/scalable/actions",
        "/io/github/hakergeniusz/Hashcheck/icons/hicolor/scalable/actions",
    ] {
        match gtk::gio::resources_enumerate_children(base, gtk::gio::ResourceLookupFlags::NONE) {
            Ok(c) => println!("{base}: {c:?}"),
            Err(e) => println!("{base}: ERR {e}"),
        }
    }
    println!(
        "has_icon hashcheck-hash-symbolic: {}",
        theme.has_icon("hashcheck-hash-symbolic")
    );

    for name in [
        "hashcheck-hash-symbolic",
        "hashcheck-file-symbolic",
        "hashcheck-success-symbolic",
        "hashcheck-error-symbolic",
    ] {
        for size in [16i32, 24, 36, 96] {
            let paintable = theme.lookup_icon(
                name,
                &[],
                size,
                1,
                gtk::TextDirection::Ltr,
                gtk::IconLookupFlags::empty(),
            );
            let file = paintable
                .file()
                .map(|f| f.uri())
                .unwrap_or_else(|| "<none>".into());
            let is_symbolic = paintable.is_symbolic();
            println!("{name} @{size}: symbolic={is_symbolic} file={file}");
        }
    }

    // Render each icon at 16 and 36 px (toggle + choose-file card sizes).
    for name in [
        "hashcheck-hash-symbolic",
        "hashcheck-file-symbolic",
        "hashcheck-success-symbolic",
        "hashcheck-error-symbolic",
    ] {
        for size in [16i32, 36] {
            let paintable = theme.lookup_icon(
                name,
                &[],
                size,
                1,
                gtk::TextDirection::Ltr,
                gtk::IconLookupFlags::empty(),
            );
            let texture = gtk::gdk::Texture::for_paintable(&paintable);
            let out = format!("/tmp/ai-agent/icon-{name}-{size}.png");
            texture.save_png(&out).expect("save png");
        }
    }
    println!("wrote /tmp/ai-agent/icon-*.png");
}
