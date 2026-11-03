//! Centralized MSRV (Minimum Supported Rust Version) constants for Dylint.
//!
//! These constants define the minimum Rust version required by Dylint and related `clippy_utils`
//! revisions. They are exported from a single location to make version updates easier and more
//! consistent across the codebase.

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
// smoelius: `zmij v1.0.1` requires Rust 1.88. However, `zmij`'s build script considers
// nightly-2025-04-22 Rust 1.88. Hence, the MSRV must be bumped to 1.89.
// smoelius: `cargo-platform@0.3.3` requires rustc 1.91.
// smoelius: Bump MSRV to get: https://github.com/rust-lang/rust/pull/158410
pub const MSRV: &str = "1.98.0";

/// The channel Clippy was using when it switched to two patch versions more than 0.[`MSRV`].
///
/// Note that Clippy versions tend to be two patch versions more than 0.<rust-version>.
pub const MSRV_CHANNEL: &str = "nightly-2026-08-20";

/// The `clippy_utils` Git OID (commit hash) when Clippy switched to two patch versions more than
/// 0.[`MSRV`].
pub const MSRV_CLIPPY_UTILS_REV: &str = "21baba9c10c5d179915cae82e6247bc2337872fb";

/// The minimum supported Rust version plus one minor version.
pub const MSRV_PLUS_1: &str = "1.99.0";

/// The channel Clippy was using when it switched to two patch versions more than 0.[`MSRV_PLUS_1`].
///
/// Note that Clippy versions tend to be two patch versions more than 0.<rust-version>.
pub const MSRV_PLUS_1_CHANNEL: &str = "nightly-2026-10-01";

/// The `clippy_utils` Git OID (commit hash) when Clippy switched to two patch versions more than
/// 0.[`MSRV_PLUS_1`].
pub const MSRV_PLUS_1_CLIPPY_UTILS_REV: &str = "a7fb0ed8a7b593daa6fb8c3d5e161cd9e0d06fb8";
