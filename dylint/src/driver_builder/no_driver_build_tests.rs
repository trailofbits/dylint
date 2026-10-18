//! Driver-build refusal tests that never run a compiler or cached driver.

use super::*;
use std::cell::Cell;

#[test]
fn refuses_missing_outdated_and_unverifiable_drivers_without_building() {
    for (status, reason) in [
        (Ok(DriverStatus::Missing), "missing"),
        (Ok(DriverStatus::Outdated), "outdated"),
        (Err(anyhow!("version query failed")), "unverifiable"),
    ] {
        let built = Cell::new(false);
        let opts = opts::Dylint {
            no_driver_build: true,
            ..Default::default()
        };
        let result = ensure_driver(&opts, Path::new("cached-driver"), status, || {
            built.set(true);
            Ok(())
        });
        assert!(!built.get(), "the {reason} driver must not be built");
        let message = result
            .err()
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();
        assert!(message.contains(reason), "{message}");
        assert!(message.contains("cached-driver"), "{message}");
        assert!(message.contains("--no-driver-build"), "{message}");
    }
}

#[test]
fn defaults_still_build_missing_outdated_and_unverifiable_drivers() {
    for status in [
        Ok(DriverStatus::Missing),
        Ok(DriverStatus::Outdated),
        Err(anyhow!("version query failed")),
    ] {
        let built = Cell::new(false);
        let result = ensure_driver(
            &opts::Dylint::default(),
            Path::new("cached-driver"),
            status,
            || {
                built.set(true);
                Ok(())
            },
        );
        assert!(
            result.is_ok(),
            "default behavior should rebuild the driver: {result:?}"
        );
        assert!(
            built.get(),
            "default behavior must invoke the build operation"
        );
    }
}

#[test]
fn current_driver_is_reused_with_or_without_refusal() {
    for no_driver_build in [false, true] {
        let opts = opts::Dylint {
            no_driver_build,
            ..Default::default()
        };
        let built = Cell::new(false);
        let result = ensure_driver(
            &opts,
            Path::new("cached-driver"),
            Ok(DriverStatus::Current),
            || {
                built.set(true);
                Ok(())
            },
        );
        assert!(
            result.is_ok(),
            "a current driver must remain usable: {result:?}"
        );
        assert!(!built.get(), "a current driver must not be rebuilt");
    }
}

#[test]
fn default_mode_propagates_build_failures() {
    let result = ensure_driver(
        &opts::Dylint::default(),
        Path::new("cached-driver"),
        Ok(DriverStatus::Missing),
        || Err(anyhow!("build failed")),
    );
    assert_eq!(
        Some("build failed"),
        result.err().as_ref().map(ToString::to_string).as_deref()
    );
}
