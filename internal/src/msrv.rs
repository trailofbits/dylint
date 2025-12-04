//! Centralized MSRV (Minimum Supported Rust Version) constants for Dylint.
//!
//! These constants define the minimum Rust version required by Dylint and related
//! `clippy_utils` revisions. They are exported from a single location to make version
//! updates easier and more consistent across the codebase.

// smoelius: I expected `git2-0.17.2` to build with nightly-2022-06-30, which corresponds to
// `--rust-version 1.64.0`. I'm not sure why it doesn't.
// smoelius: Dylint's MSRV was recently bumped to 1.68.
// smoelius: `home v0.5.9` (2013-12-15) requires rustc 1.70.0 or newer.
// smoelius: `cargo-util v0.2.7` requires rustc 1.72.0 or newer.
// smoelius: `cargo-platform v0.1.8` requires rustc 1.73 or newer.
// smoelius: `rustfix v0.8.4` requires rustc 1.75 or newer.
// smoelius: `rustfix v0.8.5` requires rustc 1.77 or newer.
// smoelius: `rustfix v0.8.6` requires rustc 1.78 or newer. However, I get errors building
// `serde` 1.0.210 with rustc 1.78, and `proc_macro2` 1.0.87 with rustc 1.79. So I am bumping
// `RUSTC_VERSION` to 1.80.
// smoelius: `home@0.5.11` (2024-12-16) requires rustc 1.81.
// smoelius: `icu_collections@2.0.0` and several other packages require rustc 1.82.
// smoelius: Edition 2024 was stabilized with Rust 1.85.
pub const MSRV: &str = "1.88.0";

/// The nightly channel date corresponding to MSRV.
pub const MSRV_CHANNEL: &str = "nightly-2025-04-22";

/// The `clippy_utils` Git OID (commit hash) for MSRV.
pub const MSRV_CLIPPY_UTILS_REV: &str = "ff428d91c2b690b8dbd8cc1e48274870c24fe1e2";

/// The minimum supported Rust version plus one minor version.
pub const MSRV_PLUS_1: &str = "1.89.0";

/// The nightly channel date corresponding to MSRV+1.
pub const MSRV_PLUS_1_CHANNEL: &str = "nightly-2025-05-14";

/// The `clippy_utils` Git OID (commit hash) for MSRV+1.
pub const MSRV_PLUS_1_CLIPPY_UTILS_REV: &str = "93bd4d893122417b9265563c037f11a158a8e37c";
