//! CLI parsing and library-option conversion for driver-build refusal.

use super::*;

#[test]
fn no_driver_build_is_explicit_and_independent_of_no_build() -> anyhow::Result<()> {
    for (arguments, no_driver_build, no_build) in [
        (vec!["cargo", "dylint"], false, false),
        (vec!["cargo", "dylint", "--no-driver-build"], true, false),
        (vec!["cargo", "dylint", "--no-build"], false, true),
        (
            vec!["cargo", "dylint", "--no-driver-build", "--no-build"],
            true,
            true,
        ),
    ] {
        let parsed = Opts::try_parse_from(arguments)?;
        let CargoSubcommand::Dylint(cli) = parsed.subcmd;
        let opts = dylint::opts::Dylint::from(cli);
        assert_eq!(opts.no_driver_build, no_driver_build);
        let dylint::opts::Operation::Check(check) = opts.operation else {
            anyhow::bail!("the default operation should remain checking");
        };
        assert_eq!(check.lib_sel.no_build, no_build);
    }
    Ok(())
}

#[test]
fn no_driver_build_reaches_the_list_operation() -> anyhow::Result<()> {
    for arguments in [
        vec!["cargo", "dylint", "list", "--no-driver-build"],
        vec!["cargo", "dylint", "list", "--no-driver-build", "--no-build"],
        vec!["cargo", "dylint", "list", "--no-build", "--no-driver-build"],
    ] {
        let parsed = Opts::try_parse_from(arguments)?;
        let CargoSubcommand::Dylint(cli) = parsed.subcmd;
        let opts = dylint::opts::Dylint::from(cli);
        assert!(opts.no_driver_build, "listing must retain driver refusal");
        assert!(matches!(opts.operation, dylint::opts::Operation::List(_)));
    }
    Ok(())
}

#[test]
fn no_driver_build_before_list_keeps_existing_argument_conflict() {
    let error = Opts::try_parse_from(["cargo", "dylint", "--no-driver-build", "list"])
        .expect_err("Dylint options must follow the final subcommand");
    assert_eq!(clap::error::ErrorKind::UnknownArgument, error.kind());
}

#[test]
fn help_describes_driver_refusal_separately_from_library_builds() {
    let error = Opts::try_parse_from(["cargo", "dylint", "--help"])
        .expect_err("help should exit before running Dylint");
    assert_eq!(clap::error::ErrorKind::DisplayHelp, error.kind());
    let help = error
        .to_string()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    assert!(help.contains("--no-driver-build"), "{help}");
    assert!(help.contains("unverifiable compiler drivers"), "{help}");
    assert!(help.contains("Do not build metadata entries"), "{help}");
}
