// SPDX-License-Identifier: MIT

use std::cell::RefCell;

use gtk::gio;
use gtk::prelude::*;

use crate::checksum::HashKind;
use crate::config;

thread_local! {
    static SETTINGS: Settings = Settings::new();
}

/// Forces settings to load (called once at startup on the main thread).
pub fn init() {
    SETTINGS.with(|_| {});
}

/// Runs `f` with the main-thread settings.
///
/// GSettings bindings are not `Send`, and every access happens on the main
/// thread, so a thread-local is the right home for them.
pub fn with<R>(f: impl FnOnce(&Settings) -> R) -> R {
    SETTINGS.with(f)
}

/// Handles GSettings with an in-memory fallback when the schema is not
/// installed (e.g. when running from the source tree without `make install`).
pub struct Settings {
    inner: Option<gio::Settings>,
    disabled: RefCell<Vec<HashKind>>,
}

const DEFAULT_WIDTH: i32 = 600;
const DEFAULT_HEIGHT: i32 = 460;
const DEFAULT_MAXIMIZED: bool = false;

impl Settings {
    fn new() -> Self {
        let inner = if schema_installed() {
            Some(gio::Settings::new(config::APP_ID))
        } else {
            None
        };
        let s = Self {
            inner,
            disabled: RefCell::new(Vec::new()),
        };
        s.reload_disabled();
        s
    }

    pub fn window_settings(&self) -> (i32, i32, bool) {
        match &self.inner {
            Some(gs) => (
                gs.int("window-width"),
                gs.int("window-height"),
                gs.boolean("is-maximized"),
            ),
            None => (DEFAULT_WIDTH, DEFAULT_HEIGHT, DEFAULT_MAXIMIZED),
        }
    }

    /// Saves window geometry; called on window close.
    pub fn save_window(&self, window: &gtk::Window) {
        let Some(gs) = &self.inner else {
            return;
        };
        let maximized = window.is_maximized();
        if !maximized {
            let _ = gs.set_int("window-width", window.width());
            let _ = gs.set_int("window-height", window.height());
        }
        let _ = gs.set_boolean("is-maximized", maximized);
    }

    pub fn disabled_hash_functions(&self) -> Vec<HashKind> {
        self.disabled.borrow().clone()
    }

    pub fn update_disabled_hash_functions(&self, disabled: &[HashKind]) {
        if let Some(gs) = &self.inner {
            let ids: Vec<&str> = disabled.iter().map(|k| k.id()).collect();
            let _ = gs.set_strv("disabled-hash-functions", ids);
        }
        *self.disabled.borrow_mut() = disabled.to_vec();
    }

    fn reload_disabled(&self) {
        let mut disabled = Vec::new();
        if let Some(gs) = &self.inner {
            for id in gs.strv("disabled-hash-functions").iter() {
                if let Some(kind) = HashKind::from_id(id) {
                    if kind != config::VERIFICATION_HASH_FUNCTION {
                        disabled.push(kind);
                    }
                }
            }
        }
        *self.disabled.borrow_mut() = disabled;
    }
}

fn schema_installed() -> bool {
    gio::SettingsSchemaSource::default()
        .and_then(|source| source.lookup(config::APP_ID, true))
        .is_some()
}
