// smoelius: Please keep these entries sorted by path.

// smoelius: Please try to use the last two path entries for a constant's name. (Note that there are
// some exceptions below.)

pub const CAMINO_UTF8_PATH_JOIN: [&str; 3] = ["camino", "Utf8Path", "join"];
pub const CAMINO_UTF8_PATH_NEW: [&str; 3] = ["camino", "Utf8Path", "new"];
pub const CAMINO_UTF8_PATH_BUF: [&str; 2] = ["camino", "Utf8PathBuf"];

pub const CELL_REF_CELL: [&str; 3] = ["core", "cell", "RefCell"];
pub const REF_CELL_BORROW_MUT: [&str; 4] = ["core", "cell", "RefCell", "borrow_mut"];

pub const SERDE_CORE_SERIALIZE_FIELD_STRUCT: [&str; 4] =
    ["serde_core", "ser", "SerializeStruct", "serialize_field"];
pub const SERDE_CORE_SERIALIZE_FIELD_STRUCT_VARIANT: [&str; 4] = [
    "serde_core",
    "ser",
    "SerializeStructVariant",
    "serialize_field",
];
pub const SERDE_CORE_SERIALIZE_FIELD_TUPLE_STRUCT: [&str; 4] = [
    "serde_core",
    "ser",
    "SerializeTupleStruct",
    "serialize_field",
];
pub const SERDE_CORE_SERIALIZE_FIELD_TUPLE_VARIANT: [&str; 4] = [
    "serde_core",
    "ser",
    "SerializeTupleVariant",
    "serialize_field",
];
pub const SERDE_CORE_SERIALIZE_ELEMENT: [&str; 4] =
    ["serde_core", "ser", "SerializeTuple", "serialize_element"];

pub const SERDE_CORE_SERIALIZE_STRUCT: [&str; 4] =
    ["serde_core", "ser", "Serializer", "serialize_struct"];
pub const SERDE_CORE_SERIALIZE_STRUCT_VARIANT: [&str; 4] = [
    "serde_core",
    "ser",
    "Serializer",
    "serialize_struct_variant",
];
pub const SERDE_CORE_SERIALIZE_TUPLE: [&str; 4] =
    ["serde_core", "ser", "Serializer", "serialize_tuple"];
pub const SERDE_CORE_SERIALIZE_TUPLE_STRUCT: [&str; 4] =
    ["serde_core", "ser", "Serializer", "serialize_tuple_struct"];
pub const SERDE_CORE_SERIALIZE_TUPLE_VARIANT: [&str; 4] =
    ["serde_core", "ser", "Serializer", "serialize_tuple_variant"];

pub const ENV_REMOVE_VAR: [&str; 3] = ["std", "env", "remove_var"];
pub const ENV_SET_CURRENT_DIR: [&str; 3] = ["std", "env", "set_current_dir"];
pub const ENV_SET_VAR: [&str; 3] = ["std", "env", "set_var"];
pub const ENV_VAR: [&str; 3] = ["std", "env", "var"];

pub const FS_COPY: [&str; 3] = ["std", "fs", "copy"];
pub const FS_DIR_ENTRY: [&str; 3] = ["std", "fs", "DirEntry"];
pub const FS_CREATE_DIR: [&str; 3] = ["std", "fs", "create_dir"];
pub const FS_CREATE_DIR_ALL: [&str; 3] = ["std", "fs", "create_dir_all"];
pub const FS_HARD_LINK: [&str; 3] = ["std", "fs", "hard_link"];
pub const FS_REMOVE_DIR: [&str; 3] = ["std", "fs", "remove_dir"];
pub const FS_REMOVE_DIR_ALL: [&str; 3] = ["std", "fs", "remove_dir_all"];
pub const FS_REMOVE_FILE: [&str; 3] = ["std", "fs", "remove_file"];
pub const FS_RENAME: [&str; 3] = ["std", "fs", "rename"];
pub const FS_SET_PERMISSIONS: [&str; 3] = ["std", "fs", "set_permissions"];
pub const FS_SOFT_LINK: [&str; 3] = ["std", "fs", "soft_link"];
pub const FS_WRITE: [&str; 3] = ["std", "fs", "write"];

pub const IO_ERROR: [&str; 4] = ["std", "io", "error", "Error"];

pub const PATH_JOIN: [&str; 4] = ["std", "path", "Path", "join"];
pub const PATH_NEW: [&str; 4] = ["std", "path", "Path", "new"];
pub const PATH_PATH_BUF: [&str; 3] = ["std", "path", "PathBuf"];

pub const COMMAND_NEW: [&str; 4] = ["std", "process", "Command", "new"];
pub const COMMAND_ARG: [&str; 4] = ["std", "process", "Command", "arg"];
pub const COMMAND_ARGS: [&str; 4] = ["std", "process", "Command", "args"];

pub const TEST_DESC_AND_FN: [&str; 3] = ["test", "types", "TestDescAndFn"];

pub const WALKDIR_DIR_ENTRY: [&str; 3] = ["walkdir", "dent", "DirEntry"];
