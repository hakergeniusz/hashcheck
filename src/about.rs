// SPDX-License-Identifier: MIT

use adw::prelude::*;

use crate::config;

pub fn show_dialog(app: &adw::Application) {
    let dialog = adw::AboutDialog::builder()
        .application_name("Hashcheck")
        .application_icon(config::APP_ID)
        .version(config::VERSION)
        .copyright("© 2026 hakergeniusz")
        .website(config::REPO_URL)
        .issue_url(config::ISSUE_URL)
        .developer_name("hakergeniusz")
        .license_type(gtk::License::MitX11)
        .build();
    dialog.present(app.active_window().as_ref());
}

pub fn show_shortcuts(app: &adw::Application) {
    let dialog = adw::ShortcutsDialog::new();
    let section = adw::ShortcutsSection::new(Some("General"));
    for (title, accelerator) in [
        ("Open File", "<Ctrl>O"),
        ("New Window", "<Ctrl>N"),
        ("Close Window", "<Ctrl>W"),
        ("Quit", "<Ctrl>Q"),
        ("Show About Dialog", "F1"),
    ] {
        section.add(adw::ShortcutsItem::new(title, accelerator));
    }
    dialog.add(section);
    dialog.present(app.active_window().as_ref());
}
