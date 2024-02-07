#[allow(clippy::struct_excessive_bools)]
#[derive(Clone, Debug, Default)]
// smoelius: Please keep the fields `names` and `args` last. Please keep all other fields sorted.
pub struct Dylint {
    pub all: bool,

    #[deprecated]
    pub allow_downgrade: bool,

    #[deprecated]
    pub bisect: bool,

    pub branch: Option<String>,

    pub fix: bool,

    #[deprecated]
    pub force: bool,

    pub git: Option<String>,

    #[deprecated]
    pub isolate: bool,

    pub keep_going: bool,

    pub lib_paths: Vec<String>,

    pub libs: Vec<String>,

    #[deprecated]
    pub list: bool,

    pub manifest_path: Option<String>,

    #[deprecated]
    pub new_path: Option<String>,

    pub no_build: bool,

    pub no_deps: bool,

    pub no_metadata: bool,

    pub packages: Vec<String>,

    pub paths: Vec<String>,

    pub pattern: Option<String>,

    pub pipe_stderr: Option<String>,

    pub pipe_stdout: Option<String>,

    pub quiet: bool,

    pub rev: Option<String>,

    #[deprecated]
    pub rust_version: Option<String>,

    pub tag: Option<String>,

    #[deprecated]
    pub upgrade_path: Option<String>,

    pub workspace: bool,

    #[deprecated]
    pub names: Vec<String>,

    pub args: Vec<String>,
}

impl Dylint {
    pub(crate) fn git_or_path(&self) -> bool {
        self.git.is_some() || !self.paths.is_empty()
    }
}
