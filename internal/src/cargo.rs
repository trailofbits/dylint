use crate::env;

pub trait SanitizeEnvironment {
    fn sanitize_environment(&mut self) -> &mut Self;
}

impl SanitizeEnvironment for crate::Command {
    fn sanitize_environment(&mut self) -> &mut Self {
        self.env_remove(env::RUSTUP_TOOLCHAIN);
        self
    }
}

pub fn build() -> crate::Command {
    cargo("build")
}

pub fn check() -> crate::Command {
    cargo("check")
}

pub fn test() -> crate::Command {
    cargo("test")
}

fn cargo(subcommand: &str) -> crate::Command {
    let mut command = crate::Command::new("cargo");
    command.args(&[subcommand]);
    command
}
