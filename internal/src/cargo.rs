#[must_use]
pub fn build() -> crate::Command {
    cargo("build")
}

#[must_use]
pub fn check() -> crate::Command {
    cargo("check")
}

#[must_use]
pub fn test() -> crate::Command {
    cargo("test")
}

fn cargo(subcommand: &str) -> crate::Command {
    let mut command = crate::Command::new("cargo");
    command.args(&[subcommand]);
    command
}
