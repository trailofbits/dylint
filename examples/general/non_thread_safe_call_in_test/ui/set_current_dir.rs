fn main() {}

#[cfg(test)]
mod test {
    use std::sync::Mutex;

    #[rustfmt::skip]
    enum Dir { North, East, South, West }

    static CURRENT_DIR: Mutex<Dir> = Mutex::new(Dir::North);

    fn set_current_dir(dir: Dir) {
        *CURRENT_DIR.lock().unwrap() = dir;
    }

    #[test]
    fn test_set_current_dir() {
        for dir in [Dir::North, Dir::East, Dir::South, Dir::West] {
            set_current_dir(dir);
        }
    }

    #[test]
    fn env_set_current_dir() {
        std::env::set_current_dir("/").unwrap();
    }
}
