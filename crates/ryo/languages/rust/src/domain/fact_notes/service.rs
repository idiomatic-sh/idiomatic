//! The `fact_notes` service: the facts of one file as the notes the parse leaves for the project to read, and the facts a note holds. The
//! bytes the notes are written as are the port `notes`'s.

use crate::domain::module_tree::constants::{CRATE, SELF, SUPER};
use crate::domain::module_tree::types::{
    Body, Cleanliness, EnclosingBlock, Facts, Gate, ModDeclaration, Segment, Spelling, Written,
};
use crate::domain::paths::types::Section;
use crate::domain::targets::types::WrittenPath;
use ryo::languages::Offset;
#[cfg(test)]
use std::borrow::Cow;

#[cfg(test)]
use super::types::{DeclaredNote, NoteFlag, NoteOffset, StepNote, WrittenNote};
use super::types::{Naming, Note, NoteName, NotePath};

/// The facts of a file as notes.
#[cfg(test)]
#[must_use]
pub(crate) fn note_of(facts: &Facts) -> Note<'_> {
    let path = |path: &Section<WrittenPath>| match path {
        Section::Present(path) => Some(NotePath(path.0.to_string())),
        Section::Absent => None,
    };
    Note {
        clean: NoteFlag(facts.clean == Cleanliness::Clean),
        declared: facts
            .declared
            .iter()
            .map(|declared| DeclaredNote {
                around: declared
                    .around
                    .iter()
                    .map(|step| StepNote {
                        name: NoteName(Cow::Borrowed(&step.name)),
                        path: path(&step.path),
                    })
                    .collect(),
                name: NoteName(Cow::Borrowed(&declared.name)),
                inline: NoteFlag(declared.body == Body::Inline),
                path: path(&declared.path),
                test_only: NoteFlag(declared.gate == Gate::Test),
            })
            .collect(),
        written: facts
            .written
            .iter()
            .map(|written| WrittenNote {
                segments: written
                    .segments
                    .iter()
                    .map(|each| NoteName(Cow::Borrowed(each)))
                    .collect(),
                enclosing: written
                    .enclosing
                    .iter()
                    .map(|each| NoteName(Cow::Borrowed(each)))
                    .collect(),
                start: NoteOffset(written.start.0),
                end: NoteOffset(written.end.0),
                text: NoteName(Cow::Borrowed(&written.text.0)),
                bound: written
                    .bound
                    .as_ref()
                    .map(|each| NoteName(Cow::Borrowed(each))),
                scope: written.scope,
            })
            .collect(),
    }
}

/// The facts a note holds, with the paths written that start with a name `names` does not hold left out: a path that starts with
/// a name no module of the repository bears, and no `crate`, `self` or `super`, names nothing of it.
#[must_use]
pub(crate) fn facts_of(note: Note<'_>, names: &dyn Fn(&Segment) -> Naming) -> Facts {
    let path = |path: Option<NotePath>| {
        path.map_or(Section::Absent, |path| {
            Section::Present(WrittenPath(path.0.into()))
        })
    };
    let segment = |name: NoteName<'_>| Segment(name.0.into());
    let declared = note
        .declared
        .into_iter()
        .map(|each| ModDeclaration {
            around: each
                .around
                .into_iter()
                .map(|step| EnclosingBlock {
                    name: segment(step.name),
                    path: path(step.path),
                })
                .collect(),
            name: segment(each.name),
            body: if each.inline.0 {
                Body::Inline
            } else {
                Body::File
            },
            path: path(each.path),
            gate: if each.test_only.0 {
                Gate::Test
            } else {
                Gate::Always
            },
        })
        .collect();
    // A name a `use` of the file binds names what that `use` names: a path that starts with it is kept to be resolved through it.
    let bound: Vec<&str> = note
        .written
        .iter()
        .filter_map(|each| each.bound.as_ref().map(|name| &*name.0))
        .collect();
    let keeps = |first: &NoteName<'_>| {
        [CRATE, SELF, SUPER].contains(&&*first.0)
            || bound.contains(&&*first.0)
            || names(&segment(first.clone())) == Naming::Known
    };
    // A `use` leaf that binds a name is kept wherever it is rooted: rooted outside the repository (`use std::fmt as helper`), it
    // names nothing, and still shadows the dependency of that name.
    let kept: Vec<bool> = note
        .written
        .iter()
        .map(|each| each.bound.is_some() || each.segments.first().is_some_and(&keeps))
        .collect();
    let written = note
        .written
        .into_iter()
        .zip(kept)
        .filter_map(|(each, kept)| kept.then_some(each))
        .map(|each| Written {
            segments: each.segments.into_iter().map(segment).collect(),
            enclosing: each.enclosing.into_iter().map(segment).collect(),
            start: Offset(each.start.0),
            end: Offset(each.end.0),
            text: Spelling(each.text.0.into()),
            bound: each.bound.map(segment),
            scope: each.scope,
        })
        .collect();
    Facts {
        clean: if note.clean.0 {
            Cleanliness::Clean
        } else {
            Cleanliness::Broken
        },
        declared,
        written,
    }
}
