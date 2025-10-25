// smoelius: This file is a slight modification of:
// https://github.com/rust-lang/cargo/blob/674e609a0ec2dc431575c48989a7bd1953ff2ab0/src/cargo/util/hex.rs

#![allow(deprecated)]
#![allow(clippy::large_stack_arrays, clippy::module_name_repetitions)]
#![cfg_attr(dylint_lib = "supplementary", allow(unnamed_constant))]
#![cfg_attr(
    dylint_lib = "non_topologically_sorted_functions",
    allow(non_topologically_sorted_functions)
)]

type StableHasher = rustc_stable_hash::StableSipHasher128;

use std::fs::File;
use std::hash::{Hash, Hasher};
use std::io::Read;

pub fn to_hex(num: u64) -> String {
    hex::encode(num.to_le_bytes())
}

pub fn hash_u64<H: Hash>(hashable: H) -> u64 {
    let mut hasher = StableHasher::new();
    hashable.hash(&mut hasher);
    Hasher::finish(&hasher)
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
    Ok(Hasher::finish(&hasher))
}

pub fn short_hash<H: Hash>(hashable: &H) -> String {
    to_hex(hash_u64(hashable))
}
