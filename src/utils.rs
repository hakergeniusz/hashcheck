// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::{BufReader, BufRead};
use std::path::Path;

use gtk::gdk;

/// Groups a hex hash by 4 characters: "01234567" -> "0123 4567".
pub fn split_by_4(hash: &str) -> String {
    hash.as_bytes()
        .chunks(4)
        .map(|c| String::from_utf8_lossy(c))
        .collect::<Vec<_>>()
        .join(" ")
}

/// Normalizes pasted hashes for comparison: lowercase, no whitespace.
pub fn normalize_input(text: &str) -> String {
    text.to_lowercase().chars().filter(|c| !c.is_whitespace()).collect()
}

/// Checks whether any whitespace-separated word in the file equals one of the
/// (lowercase) needles. Used by the compare tool for tiny files, to avoid
/// false-positives on checksum lists.
pub fn compare_content(path: &Path, needles: &[String]) -> bool {
    let Ok(file) = File::open(path) else {
        return false;
    };
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        if line
            .split_whitespace()
            .any(|word| needles.iter().any(|n| *n == word.to_lowercase()))
        {
            return true;
        }
    }
    false
}

/// The directory a file lives in, for the results status page description.
pub fn real_path(path: &Path) -> String {
    path.parent()
        .filter(|p| !p.as_os_str().is_empty())
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|| ".".to_string())
}

/// Loads the application stylesheet.
pub fn load_css() {
    let provider = gtk::CssProvider::new();
    provider.load_from_string(include_str!("../data/style.css"));
    if let Some(display) = gdk::Display::default() {
        gtk::style_context_add_provider_for_display(
            &display,
            &provider,
            gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
        );
    }
}
