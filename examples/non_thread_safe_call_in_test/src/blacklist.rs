use dylint_internal::paths;

pub const BLACKLIST: &[&[&str]] = &[
    &paths::ENV_REMOVE_VAR,
    &paths::ENV_SET_CURRENT_DIR,
    &paths::ENV_SET_VAR,
    &paths::FS_COPY,
    &paths::FS_CREATE_DIR,
    &paths::FS_CREATE_DIR_ALL,
    &paths::FS_HARD_LINK,
    &paths::FS_REMOVE_DIR,
    &paths::FS_REMOVE_DIR_ALL,
    &paths::FS_REMOVE_FILE,
    &paths::FS_RENAME,
    &paths::FS_SET_PERMISSIONS,
    &paths::FS_SOFT_LINK,
    &paths::FS_WRITE,
];
