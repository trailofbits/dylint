# Changelog

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
