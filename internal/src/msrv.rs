//! Centralized MSRV (Minimum Supported Rust Version) constants for Dylint.
//!
//! These constants define the minimum Rust version required by Dylint and related
//! clippy_utils revisions. They are exported from a single location to make version
//! updates easier and more consistent across the codebase.

/// The minimum supported Rust version for Dylint.
pub const MSRV: &str = "1.88";

/// The minimum supported Rust version plus one minor version.
pub const MSRV_PLUS_1: &str = "1.89";

/// The clippy_utils version corresponding to MSRV.
pub const MSRV_CLIPPY_UTILS_VERSION: &str = "0.1.88";

/// The clippy_utils Git OID (commit hash) for MSRV.
pub const MSRV_CLIPPY_UTILS_REV: &str = "03a5b6b976ac121f4233775c49a4bce026065b47";

/// The nightly channel date corresponding to MSRV clippy_utils.
pub const MSRV_CLIPPY_UTILS_CHANNEL: &str = "nightly-2025-05-01";

/// The clippy_utils version corresponding to MSRV+1.
pub const MSRV_PLUS_1_CLIPPY_UTILS_VERSION: &str = "0.1.89";

/// The clippy_utils Git OID (commit hash) for MSRV+1.
pub const MSRV_PLUS_1_CLIPPY_UTILS_REV: &str = "0450db33a5d8587f7c1d4b6d233dac963605766b";

/// The nightly channel date corresponding to MSRV+1 clippy_utils.
pub const MSRV_PLUS_1_CLIPPY_UTILS_CHANNEL: &str = "nightly-2025-05-14";
