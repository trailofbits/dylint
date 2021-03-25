#![feature(rustc_private)]
#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(unused_extern_crates)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_session;

use rustc_session::{config::ErrorOutputType, early_error};

use anyhow::{bail, ensure, Result};
use dylint_env::{self as env, var};
use semver::{Version, VersionReq};
use std::{ffi::OsStr, path::PathBuf};

type DylintVersionFunc = unsafe fn() -> *mut std::os::raw::c_char;

type RegisterLintsFunc =
    unsafe fn(sess: &rustc_session::Session, store: &mut rustc_lint::LintStore);

/// A library that has been loaded but whose lints have not necessarily been registered.
struct LoadedLibrary {
    path: PathBuf,
    lib: libloading::Library,
}

impl LoadedLibrary {
    fn register_lints(
        &self,
        sess: &rustc_session::Session,
        lint_store: &mut rustc_lint::LintStore,
    ) {
        (|| unsafe {
            if let Ok(func) = self.lib.get::<DylintVersionFunc>(b"dylint_version") {
                let s = std::ffi::CString::from_raw(func()).into_string()?;
                let req = VersionReq::parse(&s)?;
                let version = Version::parse(env!("CARGO_PKG_VERSION"))?;
                ensure!(
                    req.matches(&version),
                    "`{}` has dylint version {}, which does not match dylint_driver version {}",
                    self.path.to_string_lossy(),
                    req,
                    version
                );
            } else {
                bail!(
                    "could not find `dylint_version` in `{}`",
                    self.path.to_string_lossy()
                );
            }
            if let Ok(func) = self.lib.get::<RegisterLintsFunc>(b"register_lints") {
                func(sess, lint_store);
            } else {
                bail!(
                    "could not find `register_lints` in `{}`",
                    self.path.to_string_lossy()
                );
            }
            Ok(())
        })()
        .unwrap_or_else(|err| {
            sess.err(&err.to_string());
        });
    }
}

struct Callbacks {
    loaded_libs: Vec<LoadedLibrary>,
}

impl Callbacks {
    // smoelius: Load the libraries when Callbacks is created and not later (e.g., in `config`)
    // to ensure that the libraries live long enough.
    fn new(paths: Vec<PathBuf>) -> Self {
        let mut loaded_libs = Vec::new();
        for path in paths {
            unsafe {
                let lib = libloading::Library::new(&path).unwrap_or_else(|err| {
                    early_error(
                        ErrorOutputType::default(),
                        &format!(
                            "could not load library `{}`: {}",
                            path.to_string_lossy(),
                            err
                        ),
                    );
                });
                loaded_libs.push(LoadedLibrary { path, lib });
            }
        }
        Self { loaded_libs }
    }
}

trait Zeroable {
    fn zero(&mut self);
}

impl Zeroable for usize {
    fn zero(&mut self) {
        *self = 0;
    }
}

impl Zeroable for Option<usize> {
    fn zero(&mut self) {
        *self = Some(0);
    }
}

impl rustc_driver::Callbacks for Callbacks {
    fn config(&mut self, config: &mut rustc_interface::Config) {
        let previous = config.register_lints.take();
        let loaded_libs = self.loaded_libs.split_off(0);
        config.register_lints = Some(Box::new(move |sess, mut lint_store| {
            if let Some(previous) = &previous {
                previous(sess, lint_store);
            }
            for loaded_lib in &loaded_libs {
                loaded_lib.register_lints(sess, &mut lint_store);
            }
        }));

        // smoelius: Choose to be compatible with Clippy:
        // https://github.com/rust-lang/rust-clippy/commit/7bae5bd828e98af9d245b77118c075a7f1a036b9
        // smoelius: `Zeroable` is a hack to make the next line compile for different Rust versions:
        // https://github.com/rust-lang/rust-clippy/commit/0941fc0bb5d655cdd0816f862af8cfe70556dad6
        config.opts.debugging_opts.mir_opt_level.zero();
    }
}

pub fn dylint_driver<T: AsRef<OsStr>>(args: &[T]) -> Result<()> {
    if args.len() <= 1 || args.iter().any(|arg| arg.as_ref() == "-V") {
        println!("{} {}", env!("RUSTUP_TOOLCHAIN"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    // smoelius: By the above check, there are at least two arguments.

    if args[1].as_ref() == "rustc" {
        run(&args[2..])
    } else {
        run(&args[1..])
    }
}

pub fn run<T: AsRef<OsStr>>(args: &[T]) -> Result<()> {
    let sysroot = sysroot()?;
    let rustflags = rustflags();
    let paths = paths()?;

    let mut rustc_args = vec![
        "rustc".to_owned(),
        "--sysroot".to_owned(),
        sysroot.to_string_lossy().to_string(),
    ];
    rustc_args.extend(
        args.iter()
            .map(|s| s.as_ref().to_string_lossy().to_string()),
    );
    rustc_args.extend(rustflags);

    let mut callbacks = Callbacks::new(paths);

    // smoelius: I am not sure that this should be here. `RUST_LOG=debug cargo test` fails because
    // of the log messages.
    log::debug!("{:?}", rustc_args);

    rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks)
        .run()
        .map_err(|_| std::process::exit(1))
}

fn sysroot() -> Result<PathBuf> {
    let rustup_home = var(env::RUSTUP_HOME)?;
    let rustup_toolchain = var(env::RUSTUP_TOOLCHAIN)?;
    Ok(PathBuf::from(rustup_home)
        .join("toolchains")
        .join(rustup_toolchain))
}

fn rustflags() -> Vec<String> {
    var(env::DYLINT_RUSTFLAGS).map_or_else(
        |_| Vec::new(),
        |rustflags| rustflags.split_whitespace().map(String::from).collect(),
    )
}

fn paths() -> Result<Vec<PathBuf>> {
    let dylint_libs = var(env::DYLINT_LIBS)?;
    serde_json::from_str(&dylint_libs).map_err(Into::into)
}
