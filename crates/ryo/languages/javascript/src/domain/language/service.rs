//! The `language` service: free functions named by verb over the domain's types, with no IO and
//! no ports. It says which files are javascript, and how each is read.

use super::constants::{EXTENSIONS, MODULE_EXTENSION, SCRIPT_EXTENSION};
use crate::domain::syntax::types::{Goal, Scripting, Stretch, Written};
use ryo::languages;

/// Whether `path` names a javascript file: one ending in the language's extension.
#[must_use]
pub(crate) fn reads(path: languages::SourcePath<'_>) -> languages::Recognition {
    if EXTENSIONS
        .iter()
        .any(|extension| path.0.ends_with(extension))
    {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}

/// How the file at `path` is read: a `.mjs` file as a module, a `.cjs` file as a script, and every other file with module syntax
/// detected and JSX read.
#[must_use]
pub(crate) fn goal(path: languages::SourcePath<'_>) -> Goal {
    if path.0.ends_with(MODULE_EXTENSION) {
        Goal::Module
    } else if path.0.ends_with(SCRIPT_EXTENSION) {
        Goal::Script
    } else {
        Goal::Detected
    }
}

/// How a file is recognised: the language reads the files it owns, and a file of another format whose blocks are all script.
#[must_use]
pub(crate) fn recognized(
    own: languages::Recognition,
    blocks: Option<&[Stretch<'_>]>,
) -> languages::Recognition {
    if own == languages::Recognition::Reads
        || blocks.is_some_and(|each| each.iter().all(|block| block.written == Written::Script))
    {
        languages::Recognition::Reads
    } else {
        languages::Recognition::Declines
    }
}

/// How the file at `path` is read: as it is, or, when it is not a javascript file and carries blocks, as the script they make.
/// Everything outside the blocks reads as white space, line ends kept, so every offset and line of the tree is the file's and a line
/// comment ends at its own line, not at the next block.
#[must_use]
pub(crate) fn reading_of(
    path: languages::SourcePath<'_>,
    bytes: languages::ByteSlice<'_>,
    blocks: Option<&[Stretch<'_>]>,
) -> Scripting {
    let Some(blocks) = blocks.filter(|_| reads(path) == languages::Recognition::Declines) else {
        return Scripting::Own(goal(path));
    };
    let mut script: Vec<u8> = bytes
        .0
        .iter()
        .map(|byte| {
            if matches!(byte, b'\n' | b'\r') {
                *byte
            } else {
                b' '
            }
        })
        .collect();
    for block in blocks {
        // A block is a stretch of these same bytes, so its place is always inside the script.
        if let Some(place) = script.get_mut(block.start.0..block.start.0 + block.text.0.len()) {
            place.copy_from_slice(block.text.0);
        }
    }
    Scripting::Script(languages::Bytes(script))
}
