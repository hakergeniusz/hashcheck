// SPDX-License-Identifier: MIT

use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;
use std::sync::atomic::AtomicU32;

use adler::Adler32;
use md5::{Digest, Md5};
use sha1::Sha1;
use sha2::{Sha256, Sha512};

/// Number of currently running hash generations across all windows.
pub static GENERATION_COUNT: AtomicU32 = AtomicU32::new(0);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum HashKind {
    Md5,
    Sha1,
    Sha256,
    Sha512,
    Blake3,
    Crc32,
    Adler32,
}

impl HashKind {
    pub fn id(&self) -> &'static str {
        match self {
            HashKind::Md5 => "md5",
            HashKind::Sha1 => "sha1",
            HashKind::Sha256 => "sha256",
            HashKind::Sha512 => "sha512",
            HashKind::Blake3 => "blake3",
            HashKind::Crc32 => "crc32",
            HashKind::Adler32 => "adler32",
        }
    }

    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "md5" => Some(HashKind::Md5),
            "sha1" => Some(HashKind::Sha1),
            "sha256" => Some(HashKind::Sha256),
            "sha512" => Some(HashKind::Sha512),
            "blake3" => Some(HashKind::Blake3),
            "crc32" => Some(HashKind::Crc32),
            "adler32" => Some(HashKind::Adler32),
            _ => None,
        }
    }
}

/// Event sent from hashing workers to the UI thread.
#[derive(Debug)]
pub enum HashEvent {
    /// A hash function finished; value is the lowercase hex digest or an error.
    Done(HashKind, std::io::Result<String>),
}

enum Hasher {
    Md5(Md5),
    Sha1(Sha1),
    Sha256(Sha256),
    Sha512(Sha512),
    Blake3(blake3::Hasher),
    Crc32(crc32fast::Hasher),
    Adler32(Adler32),
}

impl Hasher {
    fn new(kind: HashKind) -> Self {
        match kind {
            HashKind::Md5 => Hasher::Md5(Md5::new()),
            HashKind::Sha1 => Hasher::Sha1(Sha1::new()),
            HashKind::Sha256 => Hasher::Sha256(Sha256::new()),
            HashKind::Sha512 => Hasher::Sha512(Sha512::new()),
            HashKind::Blake3 => Hasher::Blake3(blake3::Hasher::new()),
            HashKind::Crc32 => Hasher::Crc32(crc32fast::Hasher::new()),
            HashKind::Adler32 => Hasher::Adler32(Adler32::new()),
        }
    }

    fn update(&mut self, data: &[u8]) {
        match self {
            Hasher::Md5(h) => h.update(data),
            Hasher::Sha1(h) => h.update(data),
            Hasher::Sha256(h) => h.update(data),
            Hasher::Sha512(h) => h.update(data),
            Hasher::Blake3(h) => {
                h.update(data);
            }
            Hasher::Crc32(h) => h.update(data),
            Hasher::Adler32(h) => h.write_slice(data),
        }
    }

    fn finalize_hex(self) -> String {
        fn hex(bytes: &[u8]) -> String {
            bytes.iter().map(|b| format!("{b:02x}")).collect()
        }
        match self {
            Hasher::Md5(h) => hex(&h.finalize()),
            Hasher::Sha1(h) => hex(&h.finalize()),
            Hasher::Sha256(h) => hex(&h.finalize()),
            Hasher::Sha512(h) => hex(&h.finalize()),
            Hasher::Blake3(h) => h.finalize().to_hex().to_string(),
            // Zero-padded to 8 characters for interoperability.
            Hasher::Crc32(h) => format!("{:08x}", h.finalize()),
            Hasher::Adler32(h) => format!("{:08x}", h.checksum()),
        }
    }
}

/// Hashes a file in 1 MiB chunks and returns the lowercase hex digest.
pub fn hash_file(kind: HashKind, path: &Path) -> std::io::Result<String> {
    let mut hasher = Hasher::new(kind);
    let file = File::open(path)?;
    let mut reader = BufReader::with_capacity(1024 * 1024, file);
    let mut buf = vec![0u8; 1024 * 1024];
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hasher.finalize_hex())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write as _;
    use std::path::PathBuf;

    fn tmp_file(contents: &[u8]) -> PathBuf {
        // /tmp can have tight quotas on some systems; target/ is always writable.
        let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("target");
        path.push(format!("hashcheck-test-{}", std::process::id()));
        let mut file = File::create(&path).unwrap();
        file.write_all(contents).unwrap();
        path
    }

    #[test]
    fn known_digests() {
        // echo -n "abc" | <tool>
        let path = tmp_file(b"abc");
        let cases: &[(HashKind, &str)] = &[
            (
                HashKind::Md5,
                "900150983cd24fb0d6963f7d28e17f72",
            ),
            (
                HashKind::Sha1,
                "a9993e364706816aba3e25717850c26c9cd0d89d",
            ),
            (
                HashKind::Sha256,
                "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            ),
            (
                HashKind::Sha512,
                "ddaf35a193617abacc417349ae20413112e6fa4e89a97ea20a9eeee64b55d39a2192992a274fc1a836ba3c23a3feebbd454d4423643ce80e2a9ac94fa54ca49f",
            ),
            (
                HashKind::Blake3,
                "6437b3ac38465133ffb63b75273a8db548c558465d79db03fd359c6cd5bd9d85",
            ),
            (HashKind::Crc32, "352441c2"),
            (HashKind::Adler32, "024d0127"),
        ];
        for (kind, expected) in cases {
            assert_eq!(&hash_file(*kind, &path).unwrap(), expected, "for {kind:?}");
        }
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn split_by_4_groups() {
        assert_eq!(crate::utils::split_by_4("01234567"), "0123 4567");
        assert_eq!(crate::utils::split_by_4("abc"), "abc");
    }
}
