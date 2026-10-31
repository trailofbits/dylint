#![allow(dead_code)]

use std::{
    fs::{File, OpenOptions},
    io::Result,
    path::Path,
};

pub fn lock_path(path: &Path) -> Result<File> {
    let file = open_lockable_file(path)?;
    sys::lock_exclusive(&file)?;
    Ok(file)
}

pub fn try_lock_path(path: &Path) -> Result<File> {
    let file = open_lockable_file(path)?;
    sys::try_lock_exclusive(&file)?;
    Ok(file)
}

fn open_lockable_file(path: &Path) -> Result<File> {
    if cfg!(windows) {
        OpenOptions::new()
            .create(true)
            .truncate(false)
            .write(true)
            .open(path.join("NECESSIST_LOCK"))
    } else {
        File::open(path)
    }
}

// smoelius: All of the code below this comment was copied from:
// https://github.com/rust-lang/cargo/blob/b0c9586f4cbf426914df47c65de38ea323772c74/src/cargo/util/flock.rs

#[cfg(unix)]
mod sys {
    use std::{
        fs::File,
        io::{Error, Result},
        os::unix::io::AsRawFd,
    };

    pub(super) fn lock_exclusive(file: &File) -> Result<()> {
        flock(file, libc::LOCK_EX)
    }

    pub(super) fn try_lock_exclusive(file: &File) -> Result<()> {
        flock(file, libc::LOCK_EX | libc::LOCK_NB)
    }

    fn flock(file: &File, flag: libc::c_int) -> Result<()> {
        let ret = unsafe { libc::flock(file.as_raw_fd(), flag) };
        if ret < 0 {
            Err(Error::last_os_error())
        } else {
            Ok(())
        }
    }
}

#[cfg(windows)]
mod sys {
    use std::{
        fs::File,
        io::{Error, Result},
        mem,
        os::windows::io::AsRawHandle,
    };

    use windows_sys::Win32::{
        Foundation::HANDLE,
        Storage::FileSystem::{LOCKFILE_EXCLUSIVE_LOCK, LOCKFILE_FAIL_IMMEDIATELY, LockFileEx},
    };

    pub(super) fn lock_exclusive(file: &File) -> Result<()> {
        lock_file(file, LOCKFILE_EXCLUSIVE_LOCK)
    }

    pub(super) fn try_lock_exclusive(file: &File) -> Result<()> {
        lock_file(file, LOCKFILE_EXCLUSIVE_LOCK | LOCKFILE_FAIL_IMMEDIATELY)
    }

    fn lock_file(file: &File, flags: u32) -> Result<()> {
        unsafe {
            let mut overlapped = mem::zeroed();
            let ret = LockFileEx(
                file.as_raw_handle() as HANDLE,
                flags,
                0,
                !0,
                !0,
                &mut overlapped,
            );
            if ret == 0 {
                Err(Error::last_os_error())
            } else {
                Ok(())
            }
        }
    }
}
