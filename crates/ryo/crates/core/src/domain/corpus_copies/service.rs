//! The `corpus_copies` service: free functions named by verb that list the files of a corpus's inputs.

use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::files::types::{CorpusFile, FileBytes};

use super::constants::{DASH, UNDERSCORE};
use super::types::{CopyNaming, CopyNumber, CorpusInputs, Extension, ItemName, ListShape};

/// The path of the file a copy of an item is read by.
pub(crate) fn path_of(
    item: &ItemName,
    extension: &Extension,
    naming: CopyNaming,
    number: CopyNumber,
) -> FilePath {
    let folder = item.0.replace(DASH, UNDERSCORE);
    FilePath(
        match naming {
            CopyNaming::Input => format!("{folder}/input.{}", extension.0),
            CopyNaming::Copy { digits } => {
                format!(
                    "{folder}/copy_{:0width$}.{}",
                    number.0,
                    extension.0,
                    width = digits.0
                )
            }
        }
        .into(),
    )
}

/// The files of `shape`: every input, once for each copy, with its bytes repeated.
pub(crate) fn copied(
    inputs: &CorpusInputs,
    extension: &Extension,
    shape: ListShape,
) -> Vec<CorpusFile> {
    (0..shape.copies.0)
        .flat_map(|number| {
            inputs.0.iter().map(move |(item, bytes)| CorpusFile {
                path: path_of(item, extension, shape.naming, CopyNumber(number)),
                bytes: FileBytes(bytes.0.repeat(shape.repeat.0)),
            })
        })
        .collect()
}
