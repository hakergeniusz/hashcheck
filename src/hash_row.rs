// SPDX-License-Identifier: MIT

use std::cell::Cell;
use std::rc::Rc;

use adw::prelude::*;
use gtk::glib;

use crate::checksum::HashKind;
use crate::feedback;
use crate::utils::split_by_4;

/// A results row: an AdwActionRow with a copy button, showing one hash.
#[derive(Clone)]
pub struct HashRow {
    row: adw::ActionRow,
}

impl HashRow {
    pub fn new(kind: HashKind, display_name: &str, on_copy: impl Fn(HashKind) + 'static) -> Self {
        let row = adw::ActionRow::builder()
            .title(display_name)
            .subtitle_lines(100)
            .selectable(false)
            .build();

        let btn = gtk::Button::builder()
            .icon_name("edit-copy-symbolic")
            .tooltip_text("Copy")
            .valign(gtk::Align::Center)
            .css_classes(["circular"])
            .build();

        let locked = Rc::new(Cell::new(false));
        let locked_btn = locked.clone();
        btn.connect_clicked(move |btn| {
            if locked_btn.get() {
                return;
            }
            locked_btn.set(true);
            on_copy(kind);
            btn.set_icon_name(feedback::icon(true));
            btn.add_css_class("success");

            let btn = btn.downgrade();
            let locked_reset = locked.clone();
            glib::timeout_add_local_once(
                std::time::Duration::from_millis(1100),
                move || {
                    if let Some(btn) = btn.upgrade() {
                        btn.set_icon_name("edit-copy-symbolic");
                        btn.remove_css_class("success");
                    }
                    locked_reset.set(false);
                },
            );
        });

        row.add_suffix(&btn);
        Self { row }
    }

    pub fn set_hash(&self, value: &str) {
        let subtitle = if value.len() < 8 {
            value.to_string()
        } else {
            split_by_4(value)
        };
        self.row.set_subtitle(&subtitle);
    }

    pub fn set_visible_hash(&self, visible: bool) {
        self.row.set_visible(visible);
    }

    pub fn widget(&self) -> &adw::ActionRow {
        &self.row
    }
}
