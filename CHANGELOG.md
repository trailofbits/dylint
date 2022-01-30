# Changelog

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
