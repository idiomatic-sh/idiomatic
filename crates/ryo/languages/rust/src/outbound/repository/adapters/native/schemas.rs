//! What the files of the machine give, which the language does not control, and its checked conversion into the port's own types: a
//! name or a text that is not text is refused.

use std::fs::DirEntry;

use crate::domain::paths::types::Entry;
use crate::domain::paths::types::EntryKind;
use crate::domain::paths::types::EntryName;
use crate::domain::paths::types::FileText;
use crate::domain::paths::types::Section;
use crate::outbound::repository::errors::RepositoryError;

/// One thing the machine lists in a directory.
pub(super) struct EntrySchema(DirEntry);

impl TryFrom<EntrySchema> for Entry {
    type Error = RepositoryError;

    fn try_from(schema: EntrySchema) -> Result<Self, Self::Error> {
        let entry = schema.0;
        let kind = if entry.file_type().is_ok_and(|kind| kind.is_dir()) {
            EntryKind::Directory
        } else {
            EntryKind::File
        };
        Ok(Self {
            name: EntryName(
                entry
                    .file_name()
                    .into_string()
                    .map_err(|_not_utf8| RepositoryError::NotText)?
                    .into(),
            ),
            kind,
        })
    }
}

/// The bytes of a file the machine holds.
pub(super) struct FileSchema(Vec<u8>);

impl TryFrom<FileSchema> for Section<FileText> {
    type Error = RepositoryError;

    fn try_from(schema: FileSchema) -> Result<Self, Self::Error> {
        String::from_utf8(schema.0)
            .map(|text| Section::Present(FileText(text.into())))
            .map_err(|_not_utf8| RepositoryError::NotText)
    }
}

/// The text of a file the machine holds, or why it is not text.
pub(super) fn text_of_file(bytes: Vec<u8>) -> Result<Section<FileText>, RepositoryError> {
    Section::try_from(FileSchema(bytes))
}

/// A thing the machine lists in a directory, or why it is not one.
pub(super) fn entry_of(listed: DirEntry) -> Result<Entry, RepositoryError> {
    Entry::try_from(EntrySchema(listed))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
