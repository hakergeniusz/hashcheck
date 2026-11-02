// SPDX-License-Identifier: MIT

/// Returns the correct symbolic icon for feedback based on whether the task succeeded.
pub fn icon(success: bool) -> &'static str {
    if success {
        "hashcheck-success-symbolic"
    } else {
        "hashcheck-error-symbolic"
    }
}

pub fn css_add(success: bool) -> &'static str {
    if success {
        "success"
    } else {
        "error"
    }
}

pub fn css_remove(success: bool) -> &'static str {
    if success {
        "error"
    } else {
        "success"
    }
}

pub fn title(success: bool) -> &'static str {
    if success {
        "They Match"
    } else {
        "They Don't Match"
    }
}
