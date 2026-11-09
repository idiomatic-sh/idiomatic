//! The files document of the postcard store: its wire shapes and their conversion from and into the core's tables.

use super::super::strings::schemas::{StringsSchema, TextSchema, named, text};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{FileSize, Nanoseconds};
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::StoredFile;
use std::borrow::Cow;

/// Wire representation of permission knowledge; no integer sentinel means false.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) enum ExecutableSchema {
    Unknown,
    Unsupported,
    Posix(bool),
}

/// A file: one row for each, numbered by its place in the table.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct FileSchema {
    /// Explicit permission observation in this store.
    pub(in crate::outbound::lint_store::adapters::postcard) executable: ExecutableSchema,
    /// Its path.
    pub(in crate::outbound::lint_store::adapters::postcard) path: TextSchema,
    /// What its bytes came to.
    pub(in crate::outbound::lint_store::adapters::postcard) digest: [u8; 16],
    /// What it shows other files came to.
    pub(in crate::outbound::lint_store::adapters::postcard) interface: [u8; 16],
    /// What its notes and its partly read relations came to.
    pub(in crate::outbound::lint_store::adapters::postcard) noted: [u8; 16],
    /// Its size when it was read.
    pub(in crate::outbound::lint_store::adapters::postcard) size: u64,
    /// When it was last written, when it was read.
    pub(in crate::outbound::lint_store::adapters::postcard) modified: u64,
}

/// The row of a file, its path named in the table of strings.
pub(in crate::outbound::lint_store::adapters::postcard) fn file_schema<'a>(
    strings: &mut StringsSchema<'a>,
    file: &'a StoredFile,
) -> FileSchema {
    FileSchema {
        executable: match file.executable {
            ryo_values::domain::files::types::ExecutablePermission::Unknown => {
                ExecutableSchema::Unknown
            }
            ryo_values::domain::files::types::ExecutablePermission::Unsupported => {
                ExecutableSchema::Unsupported
            }
            ryo_values::domain::files::types::ExecutablePermission::PosixExecutable => {
                ExecutableSchema::Posix(true)
            }
            ryo_values::domain::files::types::ExecutablePermission::PosixNotExecutable => {
                ExecutableSchema::Posix(false)
            }
        },
        path: text(strings, &file.path.0),
        digest: file.digest.0,
        interface: file.interface.0,
        noted: file.noted.0,
        size: file.size.0,
        modified: file.modified.0,
    }
}

/// The file a row is of, its path read from the table of strings.
pub(in crate::outbound::lint_store::adapters::postcard) fn file_from(
    strings: &[Cow<'_, str>],
    file: &FileSchema,
) -> Result<StoredFile, postcard::Error> {
    Ok(StoredFile {
        executable: match file.executable {
            ExecutableSchema::Unknown => {
                ryo_values::domain::files::types::ExecutablePermission::Unknown
            }
            ExecutableSchema::Unsupported => {
                ryo_values::domain::files::types::ExecutablePermission::Unsupported
            }
            ExecutableSchema::Posix(true) => {
                ryo_values::domain::files::types::ExecutablePermission::PosixExecutable
            }
            ExecutableSchema::Posix(false) => {
                ryo_values::domain::files::types::ExecutablePermission::PosixNotExecutable
            }
        },
        path: FilePath(named(strings, file.path)?),
        digest: Digest(file.digest),
        interface: Digest(file.interface),
        noted: Digest(file.noted),
        size: FileSize(file.size),
        modified: Nanoseconds(file.modified),
    })
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
