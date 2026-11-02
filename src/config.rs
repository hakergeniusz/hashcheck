// SPDX-License-Identifier: MIT

use crate::checksum::HashKind;

pub const APP_ID: &str = "io.github.hakergeniusz.Hashcheck";
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
pub const ARTICLE: &str = "https://en.wikipedia.org/wiki/Comparison_of_cryptographic_hash_functions";
pub const REPO_URL: &str = "https://github.com/hakergeniusz/hashcheck";
pub const ISSUE_URL: &str = "https://github.com/hakergeniusz/hashcheck/issues";

/// The hash function used for the verification (compare file) tool.
/// It cannot be disabled and should be one without collisions.
pub const VERIFICATION_HASH_FUNCTION: HashKind = HashKind::Sha256;

/// All supported hash functions, in display order.
pub const HASH_FUNCTIONS: &[(HashKind, &str)] = &[
    (HashKind::Md5, "MD5"),
    (HashKind::Sha1, "SHA1"),
    (HashKind::Sha256, "SHA256"),
    (HashKind::Sha512, "SHA512"),
    (HashKind::Blake3, "Blake3"),
    (HashKind::Crc32, "CRC32"),
    (HashKind::Adler32, "Adler32"),
];

pub fn hash_display_name(kind: HashKind) -> &'static str {
    HASH_FUNCTIONS
        .iter()
        .find(|(k, _)| *k == kind)
        .map(|(_, name)| *name)
        .unwrap_or("?")
}
