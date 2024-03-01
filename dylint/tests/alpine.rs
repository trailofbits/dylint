use dylint_internal::env;
use std::{
    env::var,
    io::{stderr, Write},
    process::Command,
};

#[test]
fn alpine() {
    let status = Command::new("which").arg("docker").status().unwrap();
    if !status.success() {
        #[allow(clippy::explicit_write)]
        writeln!(
            stderr(),
            "Skipping `alpine` test as `docker` could not be found",
        )
        .unwrap();
        return;
    }

    if var(env::CI).is_ok() {
        cargo::semi_clean().unwrap();
    }

    let status = Command::new("docker")
        .args([
            "build",
            "--progress=plain",
            "-f",
            "dylint/tests/alpine/Dockerfile",
            ".",
        ])
        .current_dir("..")
        .status()
        .unwrap();
    assert!(status.success());
}

mod cargo {
    use anyhow::{ensure, Result};
    use cargo_metadata::{Artifact, Message};
    use dylint_internal::cargo::current_metadata;
    use std::{
        collections::BTreeSet,
        fs::remove_file,
        io::BufReader,
        path::PathBuf,
        process::{Command, Stdio},
    };
    use walkdir::WalkDir;

    pub fn semi_clean() -> Result<()> {
        let compiler_artifacts = compiler_artifacts()?;

        let metadata = current_metadata()?;

        for entry in WalkDir::new(metadata.target_directory) {
            let entry = entry?;
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            if compiler_artifacts.contains(path) {
                continue;
            }
            remove_file(path)?;
        }

        Ok(())
    }

    fn compiler_artifacts() -> Result<BTreeSet<PathBuf>> {
        let mut command = Command::new("cargo")
            .args(&["test", "--message-format=json", "--no-run"])
            .stdout(Stdio::piped())
            .spawn()?;

        let mut paths = BTreeSet::new();

        let stdout = command.stdout.take().unwrap();
        let reader = BufReader::new(stdout);
        for message in cargo_metadata::Message::parse_stream(reader) {
            if let Message::CompilerArtifact(Artifact {
                executable: Some(path),
                ..
            }) = message?
            {
                paths.insert(path.as_std_path().to_path_buf());
            }
        }

        let status = command.wait()?;
        ensure!(status.success(), "command failed: {:?}", command);

        Ok(paths)
    }
}
