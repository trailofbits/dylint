# dylint_linting

[docs.rs documentation]

<!-- cargo-rdme start -->

This crate provides macros for creating [Dylint] libraries, and utilities for creating
configurable libraries.

**Contents**

- [`dylint_library!`]
- [`declare_late_lint!`, `declare_early_lint!`, `declare_pre_expansion_lint!`]
- [`impl_late_lint!`, `impl_early_lint!`, `impl_pre_expansion_lint!`]
- [`constituent` feature]
- [Configurable libraries]

## `dylint_library!`

The `dylint_library!` macro expands to the following:

```rust
#[allow(unused_extern_crates)]
extern crate rustc_driver;

#[no_mangle]
pub extern "C" fn dylint_version() -> *mut std::os::raw::c_char {
    std::ffi::CString::new($crate::DYLINT_VERSION)
        .unwrap()
        .into_raw()
}
```

If your library uses the `dylint_library!` macro and the [`dylint-link`] tool, then all you
should have to do is implement the [`register_lints`] function. See the [examples] in this
repository.

## `declare_late_lint!`, etc.

If your library contains just one lint, using `declare_late_lint!`, etc. can make your code more
concise. Each of these macros requires the same arguments as [`declare_lint!`], and wraps the
following:

- a call to `dylint_library!`
- an implementation of the `register_lints` function
- a call to `declare_lint!`
- a call to [`declare_lint_pass!`]

For example, `declare_late_lint!(vis NAME, Level, "description")` expands to the following:

```rust
dylint_linting::dylint_library!();

extern crate rustc_lint;
extern crate rustc_session;

#[no_mangle]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    dylint_linting::init_config(sess);
    lint_store.register_lints(&[NAME]);
    lint_store.register_late_pass(|_| Box::new(Name));
}

rustc_session::declare_lint!(vis NAME, Level, "description");

rustc_session::declare_lint_pass!(Name => [NAME]);
```

`declare_early_lint!` and `declare_pre_expansion_lint!` are defined similarly.

## `impl_late_lint!`, etc.

`impl_late_lint!`, etc. are like `declare_late_lint!`, etc. except:

- each calls [`impl_lint_pass!`] instead of `declare_lint_pass!`;
- each requires an additional argument to specify the value of the lint's [`LintPass`]
  structure.

That is, `impl_late_lint!`'s additional argument is what goes here:

```rust
    lint_store.register_late_pass(|_| Box::new(...));
                                               ^^^
```

An example use of `impl_pre_expansion_lint!` can be found in [`env_cargo_path`] in this
repository.

## `constituent` feature

Enabling the package-level `constituent` feature changes the way the above macros work.
Specifically, it causes them to _exclude_:

- the call to `dylint_library!`
- the use of `#[no_mangle]` just prior to the declaration of `register_lints`

Such changes facilitate inclusion of a lint declared with one of the above macros into a larger
library. That is:

- With the feature turned off, the lint can be built as a library by itself.
- With the feature turned on, the lint can be built as part of a larger library, alongside other
  lints.

The [general-purpose] and [supplementary] lints in this repository employ this technique.
That is, each general-purpose lint can be built as a library by itself, or as part of the
[`general` library]. An analogous statement applies to the supplementary lints and the
[`supplementary` library]. The `constituent` feature is the underlying mechanism that makes this
work.

## Configurable libraries

Libraries can be configured by including a `dylint.toml` file in the target workspace's root
directory. This crate provides the following functions for reading and parsing `dylint.toml`
files:

- [`config_or_default`]
- [`config`]
- [`config_toml`]
- [`init_config`]
- [`try_init_config`]

A configurable library containing just one lint will typically have a `lib.rs` file of the
following form:

```rust
dylint_linting::impl_late_lint! {
    ...,
    LintName::new()
}

// Lint configuration
#[derive(Default, serde::Deserialize)]
struct Config {
    boolean: bool,
    strings: Vec<String>,
}

// Keep a copy of the configuration in the `LintPass` structure.
struct LintName {
    config: Config,
}

// Read the configuration from the `dylint.toml` file, or use the default configuration if
// none is present.
impl LintName {
    pub fn new() -> Self {
        Self {
            config: dylint_linting::config_or_default(env!("CARGO_PKG_NAME")),
        }
    }
}
```

For a concrete example of a `lib.rs` file with this form, see the
[`non_local_effect_before_error_return`] library in this repository.

A library containing more than one lint must implement the `register_lints` function without
relying on the above macros. If the library is configurable, then its `register_lints` function
should include a call to `dylint_linting::init_config`, as in the following example:

```rust
#[no_mangle]
pub fn register_lints(sess: &rustc_session::Session, lint_store: &mut rustc_lint::LintStore) {
    // `init_config` or `try_init_config` must be called before `config_or_default`, `config`,
    // or `config_toml` is called.
    dylint_linting::init_config(sess);

    lint_store.register_lints(&[FIRST_LINT_NAME, SECOND_LINT_NAME]);

    lint_store.register_late_pass(|_| Box::new(LintPassName::new()));
}
```

Additional documentation on `config_or_default`, etc. can be found on [docs.rs].

[Configurable libraries]: #configurable-libraries
[Dylint]: https://github.com/trailofbits/dylint/tree/master
[`LintPass`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_lint/trait.LintPass.html
[`config_or_default`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.config_or_default.html
[`config_toml`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.config_toml.html
[`config`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.config.html
[`constituent` feature]: #constituent-feature
[`declare_late_lint!`, `declare_early_lint!`, `declare_pre_expansion_lint!`]: #declare_late_lint-etc
[`declare_lint!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.declare_lint.html
[`declare_lint_pass!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.declare_lint_pass.html
[`dylint-link`]: https://github.com/trailofbits/dylint/tree/master/dylint-link
[`dylint_library!`]: #dylint_library
[`env_cargo_path`]: https://github.com/trailofbits/dylint/tree/master/examples/general/env_cargo_path/src/lib.rs
[`general` library]: https://github.com/trailofbits/dylint/tree/master/examples/general/src/lib.rs
[`impl_late_lint!`, `impl_early_lint!`, `impl_pre_expansion_lint!`]: #impl_late_lint-etc
[`impl_lint_pass!`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_session/macro.impl_lint_pass.html
[`init_config`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.init_config.html
[`non_local_effect_before_error_return`]: https://github.com/trailofbits/dylint/tree/master/examples/general/non_local_effect_before_error_return/src/lib.rs
[`register_lints`]: https://doc.rust-lang.org/nightly/nightly-rustc/rustc_interface/interface/struct.Config.html#structfield.register_lints
[`supplementary` library]: https://github.com/trailofbits/dylint/tree/master/examples/supplementary/src/lib.rs
[`try_init_config`]: https://docs.rs/dylint_linting/latest/dylint_linting/fn.try_init_config.html
[docs.rs documentation]: https://docs.rs/dylint_linting/latest/dylint_linting/
[docs.rs]: https://docs.rs/dylint_linting/latest/dylint_linting/
[examples]: https://github.com/trailofbits/dylint/tree/master/examples
[general-purpose]: https://github.com/trailofbits/dylint/tree/master/examples/general
[supplementary]: https://github.com/trailofbits/dylint/tree/master/examples/supplementary

<!-- cargo-rdme end -->
