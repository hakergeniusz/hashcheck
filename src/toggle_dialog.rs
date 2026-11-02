// SPDX-License-Identifier: MIT

use std::collections::HashMap;

use adw::prelude::*;
use gtk::glib;

use crate::checksum::{HashKind, GENERATION_COUNT};
use crate::config::{self, HASH_FUNCTIONS};
use crate::settings;

pub fn show(parent: &impl IsA<gtk::Widget>) {
    let dialog = adw::PreferencesDialog::builder()
        .title("Toggle Hash Functions")
        .can_close(false)
        .build();

    let page = adw::PreferencesPage::builder()
        .description("Select which hash functions should be calculated.")
        .build();
    let group = adw::PreferencesGroup::new();

    let disabled = settings::with(|s| s.disabled_hash_functions());
    let mut rows: HashMap<HashKind, adw::SwitchRow> = HashMap::new();
    for (kind, name) in HASH_FUNCTIONS {
        let row = adw::SwitchRow::builder()
            .title(*name)
            .active(!disabled.contains(kind))
            .build();

        if *kind == config::VERIFICATION_HASH_FUNCTION {
            row.set_sensitive(false);
            row.set_active(true);
            row.set_subtitle(
                "This hash function is used for file verification and cannot be disabled",
            );
        }

        group.add(&row);
        rows.insert(*kind, row);
    }

    page.add(&group);
    dialog.add(&page);

    dialog.connect_close_attempt(glib::clone!(#[weak] dialog, move |_| {
        let new_disabled: Vec<HashKind> = rows
            .iter()
            .filter(|(kind, row)| {
                **kind != config::VERIFICATION_HASH_FUNCTION && !row.is_active()
            })
            .map(|(kind, _)| *kind)
            .collect();

        let mut current = settings::with(|s| s.disabled_hash_functions());
        current.sort();
        let mut new_sorted = new_disabled.clone();
        new_sorted.sort();
        if new_sorted == current {
            // Nothing changed, just close.
            dialog.force_close();
            return;
        }

        if GENERATION_COUNT.load(std::sync::atomic::Ordering::SeqCst) > 0 {
            dialog.add_toast(
                adw::Toast::builder()
                    .title("Wait for all ongoing operations to finish")
                    .build(),
            );
            return;
        }

        settings::with(|s| s.update_disabled_hash_functions(&new_disabled));
        dialog.force_close();
    }));

    dialog.present(Some(parent));
}
