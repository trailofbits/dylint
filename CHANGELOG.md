# Changelog

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
