// smoelius: This file is a slight modification of:
// https://github.com/rust-lang/cargo/blob/aab416f6e68d555e8c9a0f02098a24946e0725fb/src/cargo/util/hex.rs

#![allow(clippy::module_name_repetitions)]
#![cfg_attr(dylint_lib = "overscoped_allow", allow(overscoped_allow))]
#![cfg_attr(dylint_lib = "supplementary", allow(unnamed_constant))]

type StableHasher = std::hash::SipHasher;

use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;

pub fn to_hex(num: u64) -> String {
    hex::encode(num.to_le_bytes())
}

pub fn hash_u64<H: Hash>(hashable: H) -> u64 {
    let mut hasher = StableHasher::new();
    hashable.hash(&mut hasher);
    hasher.finish()
}

pub fn hash_u64_file(mut file: &File) -> std::io::Result<u64> {
    let mut hasher = StableHasher::new();
    let mut buf = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.write(&buf[..n]);
    }
    Ok(hasher.finish())
}

pub fn short_hash<H: Hash>(hashable: &H) -> String {
    to_hex(hash_u64(hashable))
}
