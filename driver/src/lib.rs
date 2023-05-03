#![feature(rustc_private)]
#![deny(clippy::expect_used)]
#![deny(clippy::unwrap_used)]
#![deny(clippy::panic)]
#![deny(unused_extern_crates)]

extern crate rustc_driver;
extern crate rustc_interface;
extern crate rustc_lint;
extern crate rustc_session;
extern crate rustc_span;

use anyhow::{bail, ensure, Result};
use dylint_internal::{env, parse_path_filename, rustup::is_rustc};
use std::{
    collections::BTreeSet,
    ffi::{CString, OsStr},
    path::{Path, PathBuf},
};

pub const DYLINT_VERSION: &str = "0.1.0";

type DylintVersionFunc = unsafe fn() -> *mut std::os::raw::c_char;

type RegisterLintsFunc =
    unsafe fn(sess: &rustc_session::Session, store: &mut rustc_lint::LintStore);

/// A library that has been loaded but whose lints have not necessarily been registered.
struct LoadedLibrary {
    path: PathBuf,
    lib: libloading::Library,
}

#[derive(Clone, Eq, Ord, PartialEq, PartialOrd)]
struct Lint {
    name: &'static str,
    level: rustc_lint::Level,
    desc: &'static str,
}

impl From<&rustc_lint::Lint> for Lint {
    fn from(lint: &rustc_lint::Lint) -> Self {
        Self {
            name: lint.name,
            level: lint.default_level,
            desc: lint.desc,
        }
    }
}

impl LoadedLibrary {
    fn register_lints(
        &self,
        sess: &rustc_session::Session,
        lint_store: &mut rustc_lint::LintStore,
    ) {
        (|| unsafe {
            if let Ok(func) = self.lib.get::<DylintVersionFunc>(b"dylint_version") {
                let dylint_version = CString::from_raw(func()).into_string()?;
                ensure!(
                    dylint_version == DYLINT_VERSION,
                    "`{}` has dylint version `{}`, but `{}` was expected",
                    self.path.to_string_lossy(),
                    dylint_version,
                    DYLINT_VERSION
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
            sess.err(err.to_string());
        });
    }
}

struct Callbacks {
    loaded_libs: Vec<LoadedLibrary>,
}

// smoelius: Use of thread local storage was added to Clippy by:
// https://github.com/rust-lang/rust-clippy/commit/3c06e0b1ce003912f8fe0536d3a7fe22558e38cf
// This results in a segfault after the Clippy library has been unloaded; see the following issue
// for an explanation as to why: https://github.com/nagisa/rust_libloading/issues/5
// The workaround I've chosen is:
// https://github.com/nagisa/rust_libloading/issues/5#issuecomment-244195096
impl Callbacks {
    // smoelius: Load the libraries when `Callbacks` is created and not later (e.g., in `config`)
    // to ensure that the libraries live long enough.
    fn new(paths: Vec<PathBuf>) -> Self {
        let mut loaded_libs = Vec::new();
        for path in paths {
            unsafe {
                // smoelius: `libloading` does not define `RTLD_NODELETE`.
                #[cfg(unix)]
                let result = libloading::os::unix::Library::open(
                    Some(&path),
                    libloading::os::unix::RTLD_LAZY
                        | libloading::os::unix::RTLD_LOCAL
                        | libc::RTLD_NODELETE,
                )
                .map(Into::into);

                #[cfg(not(unix))]
                let result = libloading::Library::new(&path);

                let lib = result.unwrap_or_else(|err| {
                    rustc_session::early_error(
                        rustc_session::config::ErrorOutputType::default(),
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

#[rustversion::before(2022-07-14)]
fn zero_mir_opt_level(config: &mut rustc_interface::Config) {
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

    // smoelius: `Zeroable` is a hack to make the next line compile for different Rust versions:
    // https://github.com/rust-lang/rust-clippy/commit/0941fc0bb5d655cdd0816f862af8cfe70556dad6
    config.opts.debugging_opts.mir_opt_level.zero();
}

// smoelius: Relevant PR and merge commit:
// - https://github.com/rust-lang/rust/pull/98975
// - https://github.com/rust-lang/rust/commit/0ed9c64c3e63acac9bd77abce62501696c390450
#[rustversion::since(2022-07-14)]
fn zero_mir_opt_level(config: &mut rustc_interface::Config) {
    config.opts.unstable_opts.mir_opt_level = Some(0);
}

impl rustc_driver::Callbacks for Callbacks {
    fn config(&mut self, config: &mut rustc_interface::Config) {
        let previous = config.register_lints.take();
        let loaded_libs = self.loaded_libs.split_off(0);
        config.register_lints = Some(Box::new(move |sess, lint_store| {
            if let Some(previous) = &previous {
                previous(sess, lint_store);
            }
            let mut before = BTreeSet::<Lint>::new();
            if list_enabled() {
                lint_store.get_lints().iter().for_each(|&lint| {
                    before.insert(lint.into());
                });
            }
            for loaded_lib in &loaded_libs {
                if let Some(path) = loaded_lib.path.to_str() {
                    sess.parse_sess
                        .file_depinfo
                        .lock()
                        .insert(rustc_span::Symbol::intern(path));
                }
                loaded_lib.register_lints(sess, lint_store);
            }
            if list_enabled() {
                let mut after = BTreeSet::<Lint>::new();
                lint_store.get_lints().iter().for_each(|&lint| {
                    after.insert(lint.into());
                });
                list_lints(&before, &after);
                std::process::exit(0);
            }
        }));

        // smoelius: Choose to be compatible with Clippy:
        // https://github.com/rust-lang/rust-clippy/commit/7bae5bd828e98af9d245b77118c075a7f1a036b9
        zero_mir_opt_level(config);
    }
}

#[must_use]
fn list_enabled() -> bool {
    env::var(env::DYLINT_LIST).map_or(false, |value| value != "0")
}

fn list_lints(before: &BTreeSet<Lint>, after: &BTreeSet<Lint>) {
    let difference: Vec<Lint> = after.difference(before).cloned().collect();

    let name_width = difference
        .iter()
        .map(|lint| lint.name.len())
        .max()
        .unwrap_or_default();

    let level_width = difference
        .iter()
        .map(|lint| lint.level.as_str().len())
        .max()
        .unwrap_or_default();

    for Lint { name, level, desc } in difference {
        println!(
            "    {:<name_width$}    {:<level_width$}    {}",
            name.to_lowercase(),
            level.as_str(),
            desc,
            name_width = name_width,
            level_width = level_width
        );
    }
}

pub fn dylint_driver<T: AsRef<OsStr>>(args: &[T]) -> Result<()> {
    if args.len() <= 1 || args.iter().any(|arg| arg.as_ref() == "-V") {
        println!("{} {}", env!("RUSTUP_TOOLCHAIN"), env!("CARGO_PKG_VERSION"));
        return Ok(());
    }

    run(&args[1..])
}

pub fn run<T: AsRef<OsStr>>(args: &[T]) -> Result<()> {
    let sysroot = sysroot().ok();
    let rustflags = rustflags();
    let paths = paths();

    let rustc_args = rustc_args(args, &sysroot, &rustflags, &paths)?;

    let mut callbacks = Callbacks::new(paths);

    // smoelius: I am not sure that this should be here. `RUST_LOG=debug cargo test` fails because
    // of the log messages.
    log::debug!("{:?}", rustc_args);

    rustc_driver::RunCompiler::new(&rustc_args, &mut callbacks)
        .run()
        .map_err(|_| std::process::exit(1))
}

fn sysroot() -> Result<PathBuf> {
    let rustup_home = env::var(env::RUSTUP_HOME)?;
    let rustup_toolchain = env::var(env::RUSTUP_TOOLCHAIN)?;
    Ok(PathBuf::from(rustup_home)
        .join("toolchains")
        .join(rustup_toolchain))
}

fn rustflags() -> Vec<String> {
    env::var(env::DYLINT_RUSTFLAGS).map_or_else(
        |_| Vec::new(),
        |rustflags| rustflags.split_whitespace().map(String::from).collect(),
    )
}

fn paths() -> Vec<PathBuf> {
    (|| -> Result<_> {
        let dylint_libs = env::var(env::DYLINT_LIBS)?;
        serde_json::from_str(&dylint_libs).map_err(Into::into)
    })()
    .unwrap_or_default()
}

fn rustc_args<T: AsRef<OsStr>, U: AsRef<str>, V: AsRef<Path>>(
    args: &[T],
    sysroot: &Option<PathBuf>,
    rustflags: &[U],
    paths: &[V],
) -> Result<Vec<String>> {
    let mut args = args.iter().peekable();
    let mut rustc_args = Vec::new();

    let first_arg = args.peek();
    if first_arg.map_or(true, |arg| !is_rustc(arg)) {
        rustc_args.push("rustc".to_owned());
    }
    if let Some(arg) = first_arg {
        if is_rustc(arg) {
            rustc_args.push(arg.as_ref().to_string_lossy().to_string());
            let _ = args.next();
        }
    }
    if let Some(sysroot) = sysroot {
        rustc_args.extend([
            "--sysroot".to_owned(),
            sysroot.to_string_lossy().to_string(),
        ]);
    }
    for path in paths {
        if let Some((name, _)) = parse_path_filename(path.as_ref()) {
            rustc_args.push(format!(r#"--cfg=dylint_lib="{name}""#));
        } else {
            bail!("could not parse `{}`", path.as_ref().to_string_lossy());
        }
    }
    rustc_args.extend(args.map(|s| s.as_ref().to_string_lossy().to_string()));
    rustc_args.extend(
        rustflags
            .iter()
            .map(|rustflag| rustflag.as_ref().to_owned()),
    );

    Ok(rustc_args)
}

#[cfg(test)]
mod test {
    #![allow(clippy::unwrap_used)]

    use super::*;

    #[test]
    fn no_rustc() {
        assert_eq!(
            rustc_args(
                &["--crate-name", "name"],
                &None,
                &[] as &[&str],
                &[] as &[&Path]
            )
            .unwrap(),
            vec!["rustc", "--crate-name", "name"]
        );
    }

    #[test]
    fn plain_rustc() {
        assert_eq!(
            rustc_args(
                &["rustc", "--crate-name", "name"],
                &None,
                &[] as &[&str],
                &[] as &[&Path]
            )
            .unwrap(),
            vec!["rustc", "--crate-name", "name"]
        );
    }

    #[test]
    fn qualified_rustc() {
        assert_eq!(
            rustc_args(
                &["/bin/rustc", "--crate-name", "name"],
                &None,
                &[] as &[&str],
                &[] as &[&Path]
            )
            .unwrap(),
            vec!["/bin/rustc", "--crate-name", "name"]
        );
    }
}
