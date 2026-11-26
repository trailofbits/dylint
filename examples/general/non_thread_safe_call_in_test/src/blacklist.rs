use dylint_internal::paths;

// Each pair gives a function path and the indices of path arguments whose filesystem state the
// call may change. If all those paths are temporary, the call cannot affect another test.
// `None` means this exception does not apply.
pub const BLACKLIST: &[(&[&str], Option<&[usize]>)] = &[
    (&paths::ENV_REMOVE_VAR, None),
    (&paths::ENV_SET_CURRENT_DIR, None),
    (&paths::ENV_SET_VAR, None),
    (&paths::FS_COPY, Some(&[1])),
    (&paths::FS_CREATE_DIR, Some(&[0])),
    (&paths::FS_CREATE_DIR_ALL, Some(&[0])),
    (&paths::FS_HARD_LINK, Some(&[0, 1])), // The source inode's link count changes.
    (&paths::FS_REMOVE_DIR, Some(&[0])),
    (&paths::FS_REMOVE_DIR_ALL, Some(&[0])),
    (&paths::FS_REMOVE_FILE, Some(&[0])),
    (&paths::FS_RENAME, Some(&[0, 1])),
    (&paths::FS_SET_PERMISSIONS, Some(&[0])),
    (&paths::FS_SOFT_LINK, Some(&[1])),
    (&paths::FS_WRITE, Some(&[0])),
    (&paths::COMMAND_NEW, None),
];
