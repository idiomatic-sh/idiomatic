//! The support of the group's tests.
use crate::domain::project_namespace::types::{EntryShape, ImportedRoot, Presence};
use ryo::languages::{FileId, FilePath};

/// Adapt an observation written over booleans (is it a file, does it exist) to the typed callback.
pub(super) fn typed(
    observe: impl Fn(&FilePath, bool) -> Option<bool>,
) -> impl Fn(&FilePath, EntryShape) -> Option<Presence> {
    move |candidate, shape| observe(candidate, shape == EntryShape::File).map(Presence::from)
}

pub(super) fn path(value: &str) -> FilePath {
    FilePath(value.into())
}

pub(super) fn root(file: u32, name: &str) -> ImportedRoot {
    ImportedRoot {
        file: FileId(file),
        root: name.into(),
        path: name.into(),
    }
}

pub(super) fn dotted(file: u32, name: &str) -> ImportedRoot {
    ImportedRoot {
        file: FileId(file),
        root: name.split('.').next().unwrap().into(),
        path: name.into(),
    }
}
