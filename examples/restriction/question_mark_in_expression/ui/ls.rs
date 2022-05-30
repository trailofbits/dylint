fn main() {
    ls().unwrap();
}

fn ls() -> Result<(), std::io::Error> {
    let path = pwd().unwrap();
    for entry in std::fs::read_dir(path)? {
        let entry = entry?;
        println!("{}", entry.path().to_string_lossy());
    }
    Ok(())
}

fn pwd() -> Result<std::path::PathBuf, std::env::VarError> {
    Ok(std::path::PathBuf::from(&std::env::var("PWD")?))
}
