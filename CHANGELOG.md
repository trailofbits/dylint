# Changelog

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
