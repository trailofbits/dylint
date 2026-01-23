# Changelog

This file records user-facing changes to packages `cargo_dylint`, `dylint`, `dylint_driver`, `dylint-link`, `dylint_linting`, and `dylint_testing`. If a change to one of those packages is missing, please [open an issue](https://github.com/trailofbits/dylint/issues).

## 5.0.0

- Fix a bug causing `dylint_testing` to make repeated failed attempts to build a driver ([#1744](https://github.com/trailofbits/dylint/pull/1744))
- BREAKING: Remove `cargo-lib` feature and support for linking Cargo as a library ([#1741](https://github.com/trailofbits/dylint/pull/1741))
- Change "No paths matched" error to "No library packages found in" when a pattern is not used ([#1748](https://github.com/trailofbits/dylint/pull/1748))
- BREAKING: Remove ability to refer to libraries with `--path` ([#1754](https://github.com/trailofbits/dylint/pull/1754))
- Upgrade dependencies ([#1728](https://github.com/trailofbits/dylint/pull/1728)), including:
  - `cargo_metadata` to version 0.23
  - `cargo-util-schema` to version 0.10
  - `git2` to version 0.20

## 4.1.2

- Hot fix for a failed `cargo publish` ([#1737](https://github.com/trailofbits/dylint/pull/1737))

## 4.1.1

- Correct error messages associated with `Cargo.toml` and `rust-toolchain` backups ([#1606](https://github.com/trailofbits/dylint/pull/1606))&mdash;thanks [@suratkhan](https://github.com/suratkhan)
- Fix typo in conditional compilation documentation ([#1627](https://github.com/trailofbits/dylint/pull/1627))&mdash;thanks [@markopiers](https://github.com/markopiers)
- Use `anstyle` instead of `ansi_term` ([#1630](https://github.com/trailofbits/dylint/pull/1630))
- Fix link in README.md ([#1642](https://github.com/trailofbits/dylint/pull/1642))
- Use `tar-fs` ([#1639](https://github.com/trailofbits/dylint/pull/1639))
- Correct instructions for using Rust Analyzer ([#1658](https://github.com/trailofbits/dylint/pull/1658))&mdash;thanks [@mondeja](https://github.com/mondeja)
- Ensure correct `cargo` is called when determining package metadata ([#1671](https://github.com/trailofbits/dylint/pull/1671))
- Fix typo in `dylint-link` description ([#1700](https://github.com/trailofbits/dylint/pull/1700))
- Upgrade dependencies ([#1728](https://github.com/trailofbits/dylint/pull/1728)), including:
  - `cargo_metadata` to version 0.22
  - `rewriter` to version 0.2
  - `toml` to version 0.9
  - `toml_edit` to version 0.23

## 4.1.0

- Bump `dylint`'s MSRV to 1.81 ([b371376](https://github.com/trailofbits/dylint/commit/b371376f293a1effdb17e17e575c6529671a2d32))
- Eliminate reliance on `dirs` ([6a3e936](https://github.com/trailofbits/dylint/commit/6a3e936f583637e37627eb8ab71bda0e211ca1d8))
- Eliminate uses of `once_cell::sync::Lazy` ([#1517](https://github.com/trailofbits/dylint/pull/1517))
- Check for both `rust-toolchain` and `rust-toolchain.toml` files when determining Rust toolchain channel ([9ac18a2](https://github.com/trailofbits/dylint/commit/9ac18a298ac16d4cc173be659dbffbfbe6d877e4))

## 4.0.1

- Account for new `rustup show active-toolchain` format. [Rustup 1.28](https://github.com/rust-lang/rustup/blob/master/CHANGELOG.md#1280---2025-03-04) had two changes affecting Dylint. One was a change in `rustup show active-toolchain`'s output format; the other was the fact that Rustup no longer automatically installs uninstalled toolchains. This release addresses the former, but not the latter, i.e., you may need to manually install toolchains until the next release. To manually install a toolchain, either `cd` to a directory containing a `rust-toolchain` file and type `rustup toolchain install`, or type `rustup toolchain install TOOLCHAIN`, where `TOOLCHAIN` is the toolchain you wish to install. ([bda492f](https://github.com/trailofbits/dylint/commit/bda492fc62b416a5de9b532bf7165742b51bd34c) and [7e104fd](https://github.com/trailofbits/dylint/commit/7e104fd8763db7f6a41e9f62cdb06064fc42efdd))&mdash;thanks [@ClSlaid](https://github.com/ClSlaid) for the fix

## 4.0.0

- BREAKING CHANGE: Work with [Cargo's new hashing algorithm](https://github.com/rust-lang/cargo/pull/14917), introduced with Rust 1.85.0. `cargo-dylint` 4 does not work with Cargo 1.84.x and earlier, and `cargo-dylint` 3.x does not work with Cargo 1.85.0. If you need to use Cargo 1.84.x or earlier, please continue to use `cargo-dylint` 3.x. If you have a long term need to use Cargo 1.84.x or earlier, please [open an issue](https://github.com/trailofbits/dylint/issues). ([#1534](https://github.com/trailofbits/dylint/pull/1534))

## 3.5.1

- Have `cargo metadata` and `cargo fetch` use the same `cargo` when building library packages. Using different `cargo`s caused the two commands to refer to different subdirectories within the user's `CARGO_HOME`, leading to "Could not determine accessed subdirectory" errors. This bug primarily affected projects with `rust-toolchain` files. ([#1519](https://github.com/trailofbits/dylint/pull/1519))

## 3.5.0

- FEATURE: The `PATH` argument in `cargo dylint upgrade PATH` is now optional. When omitted, `PATH` defaults to the current directory. ([#1508](https://github.com/trailofbits/dylint/pull/1508))&mdash;thanks [@YanVictorSN](https://github.com/YanVictorSN)

## 3.4.1

- Account for [rust-lang/rust#135880](https://github.com/rust-lang/rust/pull/135880) ([#1501](https://github.com/trailofbits/dylint/pull/1501))
- Eliminate reliance on `is-terminal` ([#1502](https://github.com/trailofbits/dylint/pull/1502))

## 3.4.0

- Add link to EuroRust 2024 video ([#1447](https://github.com/trailofbits/dylint/pull/1447))
- FEATURE: Add experimental `--auto-correct` option. Upgrading a lint's Rust toolchain often causes the lint to break because of changes to Rust APIs. When `--auto-correct` is enabled, `cargo-dylint` tries to infer lint fixes from Clippy's commit history. ([949cf8f](https://github.com/trailofbits/dylint/commit/949cf8f322bd664b3e432738700dd3ff223693c2), [625d3d3](https://github.com/trailofbits/dylint/commit/625d3d3673f22b086dfcc04af62c3b79948e2c0a), [c1edcfb](https://github.com/trailofbits/dylint/commit/c1edcfb1c67e4af61900af3d3f1c14b7688f6e77), [2753a88](https://github.com/trailofbits/dylint/commit/2753a88e7ce721689953cbe67fa9ca909e7fd570), and [c515601](https://github.com/trailofbits/dylint/commit/c5156019897b1674b6ff57ac839afe5a127a35e1))

## 3.3.0

- Add EuroRust 2024 slides to README.md ([#1370](https://github.com/trailofbits/dylint/pull/1370))
- Upgrade `cargo-util-schemas` to version 0.7 ([#1394](https://github.com/trailofbits/dylint/pull/1394) and [#1429](https://github.com/trailofbits/dylint/pull/1429))
- Upgrade `cargo_metadata` to version 0.19 ([8a7bc95](https://github.com/trailofbits/dylint/commit/8a7bc9575875c50da2acae390be0f5a611103a38))
- Bump `dylint`'s MSRV to 1.78 ([c4c27d1](https://github.com/trailofbits/dylint/commit/c4c27d1e9969e9f1c0a097223fa99fd2a30c0a6d))
- Upgrade `thiserror` to version 2 ([7398557](https://github.com/trailofbits/dylint/commit/73985574c95ba2c512df4c1b25ebe1aab0a05e92))
- Remove mention of `rustc_attr` from Dylint library template, as the crate was [recently renamed](https://github.com/rust-lang/rust/pull/134381) ([#1446](https://github.com/trailofbits/dylint/pull/1446))

## 3.2.1

- Enable drivers to be installed concurrently. Previously, an attempt to install a driver would fail if the existing driver was running. ([4438f40](https://github.com/trailofbits/dylint/commit/4438f402591d05e369636f54e18b4bab232702b5) and [231974a](https://github.com/trailofbits/dylint/commit/231974a7b94d2c22e85227998a6bc8fd94c5cd0f))
- Fix capitalization in help messages ([5a44c22](https://github.com/trailofbits/dylint/commit/5a44c2266bc1ee660791535042e4c5e57ee4feff))

## 3.2.0

- Upgrade `compiletest_rs` to version 0.11 in `dylint_testing` ([145623a](https://github.com/trailofbits/dylint/commit/145623aedba033d90e25e54b6f178178202df3d5))
- Add documentation on the example libraries' use of `dylint_linting`'s `constituent` feature ([9625952](https://github.com/trailofbits/dylint/commit/96259528055d0cad4a52c9914e056ad614c821ce))
- Add documentation on suppressing rustc's `unexpected_cfg` lint warnings ([#1243](https://github.com/trailofbits/dylint/pull/1243) and [#1252](https://github.com/trailofbits/dylint/pull/1252))
- FEATURE: Allow `pattern`s to be arrays when specifying workspace metadata entries ([#1273](https://github.com/trailofbits/dylint/pull/1273))
- Upgrade `cargo` to version 0.81 ([#1280](https://github.com/trailofbits/dylint/pull/1280))
- Add `#![feature(rustc_private)]` to `dylint_driver`'s main.rs. This fixes a bug caused by [rust-lang/rust#122362](https://github.com/rust-lang/rust/pull/122362). ([b691069](https://github.com/trailofbits/dylint/commit/b69106982158c34a1874639fa7b196a4a1e830bb))
- Enable Dylint to use Cargo's checkouts directories concurrently. Previously, such concurrent use could cause Dylint to produce errors. ([cb6299a](https://github.com/trailofbits/dylint/commit/cb6299aa10418e655e6e585b564190ad802ad535))
- FEATURE: In `dylint_testing`, `ui_test` and `Test::src_base` now accept an `impl AsRef<Path>` as opposed to a `&Path` ([be85b10](https://github.com/trailofbits/dylint/commit/be85b10a44e4fc173241a71cf7b0e1fba65d8c74))

## 3.1.2

- Extend rather than overwrite `RUSTFLAGS` when building drivers ([7662d4c](https://github.com/trailofbits/dylint/commit/7662d4c4d761aeaa47e37a74cd9133adc9b8ae59))

## 3.1.1

- Update `env_logger` to version 0.11 in `dylint_driver` and `dylint_testing` ([#1200](https://github.com/trailofbits/dylint/pull/1200))
- Prepend the toolchain `bin` directory to `PATH` on Windows (i.e., account for [rust-lang/rustup#3703](https://github.com/rust-lang/rustup/pull/3703)) ([c222181](https://github.com/trailofbits/dylint/commit/c22218107853018a04e01a65a16406612e36186a))

## 3.1.0

- FEATURE: Allow library packages to be specified in a dylint.toml file instead of a Cargo.toml file. The syntax is exactly the same. Thus, users wishing to switch can simply cut-and-paste the `[workspace.metadata.dylint.libraries]` declaration from their Cargo.toml file into a dylint.toml file. ([#1143](https://github.com/trailofbits/dylint/pull/1143) and [#1151](https://github.com/trailofbits/dylint/pull/1151))

## 3.0.1

- Address [rust-lang/rust#122450](https://github.com/rust-lang/rust/pull/122450) ([9add993](https://github.com/trailofbits/dylint/commit/9add9935d34bc58c425b5b1be0e9227c4b1f54b1))

## 3.0.0

- Rename option `--path` to `--lib-path`. For the time being, `--path` will continue to work as before when used to refer to a file as opposed to a directory. ([e52da02](https://github.com/trailofbits/dylint/commit/e52da024fada4ff4b353ca2de86ed12499076972))
- FEATURE: Add options `--git` and `--path` to allow naming library packages on the command line ([883e521](https://github.com/trailofbits/dylint/commit/883e5218734819d120c332e025aa726ab30dbd40))
- BREAKING CHANGE: Make `metadata` no longer a default `dylint` package feature ([#1052](https://github.com/trailofbits/dylint/pull/1052)) (see also renaming of `metadata` below)
- BREAKING CHANGE: No longer pass `-D warnings` to rustc by default in `dylint_testing`. To retain the previous behavior, enable the `deny_warnings` feature. ([#1053](https://github.com/trailofbits/dylint/pull/1053))
- BREAKING CHANGE: Rename the following package features ([42cb7a2](https://github.com/trailofbits/dylint/commit/42cb7a2d48b398f90bc5975c69f976f5fedd6c65)):
  - `cargo-dylint` package: `metadata-cargo` -> `cargo-lib` (download library packages using `cargo` as a library)
  - `cargo-dylint` package: `metadata-cli` -> `cargo-cli` (download library packages using the `cargo` executable)
  - `dylint` package: `metadata` -> `library_packages` (enable library-package-related functionality, e.g., building them)
- BREAKING CHANGE: Remove the following deprecated options and their associated `Dylint` struct fields ([7fd2c4d](https://github.com/trailofbits/dylint/commit/7fd2c4d410eb0b7744f26c224e1eb3c23083e551)):
  - `--allow-downgrade` (now part of the `upgrade` subcommand)
  - `--bisect` (experimental and no longer deemed necessary)
  - `--force` (renamed to `--allow-downgrade`)
  - `--isolate` (now part of the `new` subcommand)
  - `--list` (replaced with the `list` subcommand)
  - `--new` (replaced with the `new` subcommand)
  - `--rust-version` (now part of the `upgrade` subcommand)
  - `--upgrade` (replaced with the `upgrade` subcommand)
- Update `cargo-dylint` MSRV to 1.74 ([965ebf5](https://github.com/trailofbits/dylint/commit/965ebf58d280db22a8143cbfbe88c17dea4f5617))
- Eliminate reliance on `sedregex` ([#1079](https://github.com/trailofbits/dylint/pull/1079))
- Make `cargo-cli` the default method for building library packages ([01aa9ba](https://github.com/trailofbits/dylint/commit/01aa9ba790ff15c3f3e20a71a455bee5424aff09))

## 2.6.1

- Fix two bugs ([#1014](https://github.com/trailofbits/dylint/issues/1014) and [#1021](https://github.com/trailofbits/dylint/issues/1021)) related to how warnings are displayed ([#1025](https://github.com/trailofbits/dylint/pull/1025))

## 2.6.0

- Add experimental `metadata-cli` feature ([#926](https://github.com/trailofbits/dylint/pull/926), [#976](https://github.com/trailofbits/dylint/pull/976), and [#1008](https://github.com/trailofbits/dylint/pull/1008))

## 2.5.0

- Allow the `list` subcommand to accept `--manifest-path` options ([#951](https://github.com/trailofbits/dylint/pull/951))

## 2.4.4

- Fix [broken links](https://github.com/trailofbits/dylint/issues/691) in docs.rs documentation ([0f04fcb](https://github.com/trailofbits/dylint/commit/0f04fcb4f955639711ba3fe12c4994246c96dadb))

## 2.4.3

- [#858](https://github.com/trailofbits/dylint/pull/858) made `curl-sys` a non-optional dependency of the `dylint` package. This had unintended side effects, e.g., causing all of the example lints to transitively depend on `curl-sys`. [#867](https://github.com/trailofbits/dylint/pull/867) corrects the situation. ([#867](https://github.com/trailofbits/dylint/pull/867))
- Ensure that consecutive uses of `--lib` produce the correct output, i.e., are not improperly cached ([#866](https://github.com/trailofbits/dylint/pull/866))&mdash;thanks [@EFanZh](https://github.com/EFanZh) for the [bug report](https://github.com/trailofbits/dylint/issues/856)

## 2.4.2

- Work around [curl/curl#11893](https://github.com/curl/curl/issues/11893) ([#858](https://github.com/trailofbits/dylint/pull/858))&mdash;thanks [@tjade273](https://github.com/tjade273) for the [bug report](https://github.com/trailofbits/dylint/issues/857)

## 2.4.1

- Update dependencies, including `cargo` to version 0.73.1 ([#847](https://github.com/trailofbits/dylint/pull/847))

## 2.4.0

- Update `cargo-dylint` and `dylint` MSRVs ([d42a7a0](https://github.com/trailofbits/dylint/commit/d42a7a076b5f4118fc9dffc9b932f228d4a79bc9))
- Specify MSRV policy ([#835](https://github.com/trailofbits/dylint/pull/835))
- Make `cargo dylint update` work when `clippy_utils` is a workspace dependency, as opposed to just when it is a package dependency (bugfix) ([36d94ed](https://github.com/trailofbits/dylint/commit/36d94ed9f8a6f190e7f90d064846585b2a69c8fc))
- Don't canonicalize paths passed to Cargo in `dylint_linting` (bugfix) ([d6b0356](https://github.com/trailofbits/dylint/commit/d6b0356f2457c41eb15207af08e8e80b5d29f525))

## 2.3.0

- Add `--pipe-stderr` and `--pipe-stdout` options ([#822](https://github.com/trailofbits/dylint/pull/822))&mdash;thanks [@faculerena](https://github.com/faculerena)

## 2.2.0

- Add `constituent` feature to facilitate building a lint by itself, or as part of a larger library ([#790](https://github.com/trailofbits/dylint/pull/790) and [#812](https://github.com/trailofbits/dylint/pull/812))
- When building metadata entries, ignore subdirectories that do not contain packages rather than generate errors ([#809](https://github.com/trailofbits/dylint/pull/809))
- Add `--no-deps` option ([#808](https://github.com/trailofbits/dylint/pull/808))&mdash;thanks [@EFanZh](https://github.com/EFanZh) for the [suggestion](https://github.com/trailofbits/dylint/issues/804)
- Rerun lints when workspace metadata changes ([#813](https://github.com/trailofbits/dylint/pull/813))&mdash;thanks [@maxammann](https://github.com/maxammann) for the [bug report](https://github.com/trailofbits/dylint/issues/650)

## 2.1.12

- Update `cargo` to version 0.72.1 ([#786](https://github.com/trailofbits/dylint/pull/786))

## 2.1.11

- Address [rust-lang/rust#112692](https://github.com/rust-lang/rust/pull/112692) ([f4094c8](https://github.com/trailofbits/dylint/commit/f4094c82229ea79a764e9dbf644e28feb8997dd1))
- Migrate away from `atty` ([#736](https://github.com/trailofbits/dylint/pull/736))

## 2.1.10

- Address [rust-lang/rust#111748](https://github.com/rust-lang/rust/pull/111748) ([#699](https://github.com/trailofbits/dylint/pull/699))
- Fix a bug causing `dylint_testing` to fail to determine `rustc` flags on Windows ([730696a](https://github.com/trailofbits/dylint/commit/730696a4438b11f9bca8b174500f4ae11eb75419))
- Eliminate last reference to `syn` 1.0 ([#709](https://github.com/trailofbits/dylint/pull/709))

## 2.1.9

- Address [rust-lang/rust#111633](https://github.com/rust-lang/rust/pull/111633) ([#694](https://github.com/trailofbits/dylint/pull/694))

## 2.1.8

- Allow libraries to use thread local storage ([3db9dda](https://github.com/trailofbits/dylint/commit/3db9dda14ecb939ff019f1b6c27c5515df91bfaa))
- Treat unknown workspace metadata keys as errors ([95b4bbc](https://github.com/trailofbits/dylint/commit/95b4bbc09c621c28042a0eb4b71e15107ca7052c))
- Fix a bug causing false "invalid pattern" errors on Windows ([49e0353](https://github.com/trailofbits/dylint/commit/49e0353b3a590554d0ee79ff1f86f5ed24e1039c))
- Don't treat unbuilt libraries as errors when listing libraries ([75190fb](https://github.com/trailofbits/dylint/commit/75190fbdc39cc29b152737a32959d00c17980b2e))

## 2.1.7

- Update dependencies, including `openssl` to version 0.10.48 ([#652](https://github.com/trailofbits/dylint/pull/652))

## 2.1.6

- Enable backtraces for stable builds ([#630](https://github.com/trailofbits/dylint/pull/630))
- Dylint now builds libraries only when they are needed to run. For example, `cargo dylint --lib foo` builds just library `foo`, whereas it used to build all available libraries. ([#633](https://github.com/trailofbits/dylint/pull/633))

## 2.1.5

- Use [`home`](https://crates.io/crates/home) crate to determine `CARGO_HOME` ([#604](https://github.com/trailofbits/dylint/pull/604))
- When `dylint.toml` cannot be parsed, show reason why ([d6f9d5f](https://github.com/trailofbits/dylint/commit/d6f9d5fa03d0c3fe23db66dc8f2361605c4a59fe))
- Update `tempfile` to version 3.4.0 ([#624](https://github.com/trailofbits/dylint/pull/624))

## 2.1.4

- Address [rust-lang/rust#106810](https://github.com/rust-lang/rust/pull/106810) ([#590](https://github.com/trailofbits/dylint/pull/590))

## 2.1.3

- Set rust-analyzer's `rustc_private=true` in `dylint_linting` package metadata ([#543](https://github.com/trailofbits/dylint/pull/543))

## 2.1.2

- Rerun `cargo check` when library code changes ([6235e99](https://github.com/trailofbits/dylint/commit/6235e9993aa374a8a568fbbda4c333d718985835))
- Clear `RUSTFLAGS` when building workspace metadata entries ([a5f0d4f](https://github.com/trailofbits/dylint/commit/a5f0d4ffe13b20a29681759191275456a3cd236b))
- Fix help messages ([cc10586](https://github.com/trailofbits/dylint/commit/cc105862ddd78ddf3379a83e27401100d6242fa5) and [9d60366](https://github.com/trailofbits/dylint/commit/9d603667cd5096fea01b7c635d1d24cceea73ade))
- Fix rust-analyzer configuration in [VS Code integration](https://github.com/trailofbits/dylint#vs-code-integration) section of README ([#540](https://github.com/trailofbits/dylint/pull/540))&mdash;thanks [@fcasal](https://github.com/fcasal)

## 2.1.1

- Fix a bug that would cause `dylint_linting::init_config` to fail when run on a build.rs file in the workspace root ([#503](https://github.com/trailofbits/dylint/pull/503))

## 2.1.0

- Allow libraries to be configured via a `dylint.toml` file in a workspace's root directory ([#484](https://github.com/trailofbits/dylint/pull/484) and [#496](https://github.com/trailofbits/dylint/pull/496))&mdash;thanks [@shepmaster](https://github.com/shepmaster) for the [suggestion](https://github.com/trailofbits/dylint/issues/482)

## 2.0.14

- Prevent patterns from escaping dependency directories ([#486](https://github.com/trailofbits/dylint/pull/486))&mdash;thanks [@shepmaster](https://github.com/shepmaster) for the [bug report](https://github.com/trailofbits/dylint/issues/485)

## 2.0.13

- Support [rust-lang/rust#101501](https://github.com/rust-lang/rust/pull/101501) in `dylint_linting` ([06bdab3](https://github.com/trailofbits/dylint/commit/06bdab31922b6019757e715896077265e1d0d764))
- Update library package template ([f119c03](https://github.com/trailofbits/dylint/commit/f119c037eff9acea85a91f1f37512d53157b327a) and [256aa92](https://github.com/trailofbits/dylint/commit/256aa927e15e205b7e087f4ae36be4cba4503e92))

## 2.0.12

- Strip current directory when listing libraries ([7268b0a](https://github.com/trailofbits/dylint/commit/7268b0aaedf6b8d52a3e9bf8c5ba24a8a4cd94c6))
- Switch to subcommands ([4b240bc](https://github.com/trailofbits/dylint/commit/4b240bc5037a0feb7317f21a20445bd6e9d54f0c))
- Clone with CLI by default ([#434](https://github.com/trailofbits/dylint/pull/434))
- Upgrade library packages using [`toml_edit`](https://github.com/ordian/toml_edit) ([#436](https://github.com/trailofbits/dylint/pull/436))
- Ensure dylint_driver_manifest_dir.rs is truncated in dylint/build.rs ([524850b](https://github.com/trailofbits/dylint/commit/524850baafebfa62d578d498b660bc0011826bc6))

## 2.0.11

- Fix bug related to package cache locking ([#421](https://github.com/trailofbits/dylint/pull/421))

## 2.0.10

- Allow installation with `--debug` ([6b6e34e](https://github.com/trailofbits/dylint/commit/6b6e34e408f0bb132b6549b062cb71bab63dddfc))
- Fix missing `RUSTUP_TOOLCHAIN` environment variable bug affecting Windows ([f5cb5b7](https://github.com/trailofbits/dylint/commit/f5cb5b765573526bb08255a6c905c363ce461243))
- Update library package template ([e59ac2f](https://github.com/trailofbits/dylint/commit/e59ac2fb61f976c7516d3bc8759b85759b111a4d))
- Sort not found libraries ([79a7171](https://github.com/trailofbits/dylint/commit/79a71715d795d5d17536646f29f4534d161b7e45))
- Retry failing `git clone`s ([#395](https://github.com/trailofbits/dylint/pull/395))
- Update template ([#396](https://github.com/trailofbits/dylint/pull/396))

## 2.0.9

- Make driver work with latest nightly ([#381](https://github.com/trailofbits/dylint/pull/381))
- Fix bug in how driver ordered arguments passed to rustc ([#385](https://github.com/trailofbits/dylint/pull/385))

## 2.0.8

- Clear `RUSTC` environment variable when building metadata entries and when running `cargo check`/`fix` ([#379](https://github.com/trailofbits/dylint/pull/379))

## 2.0.7

- Report all not found libraries, not just the first one ([#350](https://github.com/trailofbits/dylint/pull/350))
- No longer use [`dylint-template`](https://github.com/trailofbits/dylint-template) to create new libraries ([#355](https://github.com/trailofbits/dylint/pull/355))

## 2.0.6

- If target triple cannot be determined from toolchain, default to host triple ([ff4a069](https://github.com/trailofbits/dylint/commit/ff4a069800c9e6d8d33ff0ed03442343234cbe9f))
- Relax restriction that library be in its own workspace ([167ce9e](https://github.com/trailofbits/dylint/commit/167ce9ed1b1f37718e83f32a4314ac1cf0dd5909))
- Ensure library filename uses snake case ([54f3fb6](https://github.com/trailofbits/dylint/commit/54f3fb69426007ca794cdfe9f8b9ebad1600d1a7))

## 2.0.5

- Fix a bug that was causing `rustfix` to not work with example tests ([#341](https://github.com/trailofbits/dylint/pull/341))

## 2.0.4

- Respect `linker` setting in `$CARGO_HOME/config.toml` ([#339](https://github.com/trailofbits/dylint/pull/339))

## 2.0.3

- Error when metadata entry names a nonexistent library ([#317](https://github.com/trailofbits/dylint/pull/317))
- Enable conditional compilation ([#322](https://github.com/trailofbits/dylint/pull/322))&mdash;idea due to [@danielhenrymantilla](https://github.com/danielhenrymantilla) ([#28](https://github.com/trailofbits/dylint/issues/28))
- Rename `--force` to `--allow-downgrade` ([#331](https://github.com/trailofbits/dylint/pull/331) and [#333](https://github.com/trailofbits/dylint/pull/333))

## 2.0.2

- Make `--new` work with new macros ([#298](https://github.com/trailofbits/dylint/pull/298))

## 2.0.1

- Add macros [`declare_late_lint!`, etc.](https://github.com/trailofbits/dylint/tree/master/utils/linting#declare_late_lint-etc) to `dylint_linting`. The new macros make it easier to write libraries containing just one lint (the current common case). ([#284](https://github.com/trailofbits/dylint/pull/284))
- Don't iterate over `name_toolchain_map` to list lints ([1ad7da7](https://github.com/trailofbits/dylint/commit/1ad7da7cace2089231cb95e9a58515f1e2b712d6))

## 2.0.0

- Use correct crate names ([b728be3](https://github.com/trailofbits/dylint/commit/b728be3b47b496cdbbcb0e27cc954f3fabf4a189))
- Adjust message displayed when examples are rebuilt ([e7ae412](https://github.com/trailofbits/dylint/commit/e7ae412d29edf69bcbb84d4f8d1cc9baf959f1d4))
- BREAKING CHANGE: Build `name_toolchain_map` on first use. For example, if all libraries are specified with `--path`, then there is no need to build the target's metadata entries. The `name_toolchain_map` is technically part of Dylint's public API. Hence, this is a breaking change. ([#250](https://github.com/trailofbits/dylint/pull/250))

## 1.0.14

- Add test "builder" to `dylint_testing` ([#222](https://github.com/trailofbits/dylint/pull/222) and [#237](https://github.com/trailofbits/dylint/pull/237))
- Determine `clippy_utils` versions using commit history rather than git tags ([#236](https://github.com/trailofbits/dylint/pull/236))
- Ensure package cache is locked ([#247](https://github.com/trailofbits/dylint/pull/247))
- Verify build succeeded before considering bisect successful ([#246](https://github.com/trailofbits/dylint/pull/246))
- Eliminate redundant builds when using `dylint_testing` ([#216](https://github.com/trailofbits/dylint/pull/216))

## 1.0.13

- Hide `cargo-bisect-rustc`'s progress bars when `--quiet` is passed or when not on a tty ([#214](https://github.com/trailofbits/dylint/pull/214))

## 1.0.12

- If an `--upgrade` would result in a downgrade, and `--bisect` is passed, the downgrade is skipped and the bisect proceeds instead of producing an error. ([#183](https://github.com/trailofbits/dylint/pull/183))
- Be more explicit about what cargo is doing ([#185](https://github.com/trailofbits/dylint/pull/185))

## 1.0.11

- Add experimental `--bisect` option ([#170](https://github.com/trailofbits/dylint/pull/170))

## 1.0.10

- Add `--fix` option ([#153](https://github.com/trailofbits/dylint/pull/153))
- Prevent `--upgrade` from downgrading toolchain ([#164](https://github.com/trailofbits/dylint/pull/164))
- Expand circumstances under which drivers are rebuilt ([#165](https://github.com/trailofbits/dylint/pull/165))

## 1.0.9

- Update clap dependency ([#152](https://github.com/trailofbits/dylint/pull/152))

## 1.0.8

- Separate compilation artifacts by toolchain ([28f3691](https://github.com/trailofbits/dylint/commit/28f3691221bc22047b9bc6d7fcefa72b038adc10))
- Add `--keep-going` option ([bbf0a3c](https://github.com/trailofbits/dylint/commit/bbf0a3c964788e86a287b49c8a9b1d5315c315e3))

## 1.0.7

- Update clap dependency ([#104](https://github.com/trailofbits/dylint/pull/104))

## 1.0.6

- Add `--new` and `--upgrade` options ([#92](https://github.com/trailofbits/dylint/pull/92))
- Improve error messages ([#103](https://github.com/trailofbits/dylint/pull/103))

## 1.0.5

- Improve build times when testing libraries ([e5ce5b9](https://github.com/trailofbits/dylint/commit/e5ce5b9583d09c57ac177bdf9f05ffe06c6550c6)) and when linting using workspace metadata ([7204bce](https://github.com/trailofbits/dylint/commit/7204bce39dc4540601e4548e695d9176e3527ed1))

## 1.0.4

- Better handling of target directories ([#77](https://github.com/trailofbits/dylint/pull/77))
- Hide `dylint_version()` in docs ([#78](https://github.com/trailofbits/dylint/pull/78))&mdash;thanks [@MinerSebas](https://github.com/MinerSebas)

## 1.0.3

- Update clap dependency ([#65](https://github.com/trailofbits/dylint/pull/65))

## 1.0.2

- Link Dylint drivers using absolute paths (fixes [#54](https://github.com/trailofbits/dylint/issues/54))
- Windows support (thanks to [@MinerSebas](https://github.com/MinerSebas))

## 1.0.1

- Bug fixes ([#38](https://github.com/trailofbits/dylint/issues/38), [#39](https://github.com/trailofbits/dylint/issues/39))

## 1.0.0

- Add support for [workspace metadata](./README.md#workspace-metadata)
- BREAKING CHANGE: No longer search `target/debug` and `target/release` for libraries

## 0.1.3

- Add `ui_test_example` and `ui_test_examples` ([#20](https://github.com/trailofbits/dylint/pull/20))

## 0.1.2

- Use rust-toolchain to build drivers ([c28639ee](https://github.com/trailofbits/dylint/commit/c28639eecefb88c2d95e67527239600867b04757))

## 0.1.1

- Fetch remote `dylint_driver` in `dylint_testing` by default ([#15](https://github.com/trailofbits/dylint/pull/15))

## 0.1.0

- Initial release
