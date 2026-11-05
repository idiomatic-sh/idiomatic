//! Reading a source with a language's own parse, for the tests of the items of a relation: the rows the language writes of the item's
//! relation, each in full as typed cells, and the causes for which it says the relation is read only in part.

use super::super::errors::ParseError;
use super::super::service::read;
use super::super::types::ParseEngine;
use super::super::types::{FieldRequest, NoteRequest, ParseRequest, SampleText};
use super::contract::Collect;
use super::expected::{Wanted, kept};
use super::rows::{Read, rows_of_relation};
use crate::domain::elements::types::Elements;
use crate::domain::files::types::SourcePath;
use ryo_relations::domain::relations::types::{FileId, Relation};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::terms::types::UnknownName;

/// What a language wrote of one relation for a source.
#[derive(Clone, Debug, PartialEq, Eq, Copy)]
pub struct Reading {
    /// The rows of the relation, every cell of each, in the order of the cells.
    pub rows: Wanted,
    /// The causes for which the relation is read only in part, once each, in the order they were first said.
    pub causes: &'static [&'static str],
}

/// What `language` writes of `relation` for `source`, read as the file numbered 0 at `path`, asking for the roles of the elements.
///
/// # Errors
///
/// When the language refuses the source.
pub fn reading(
    language: &dyn ParseEngine,
    path: SourcePath<'_>,
    source: SampleText,
    relation: Relation,
) -> Result<Reading, ParseError> {
    let mut symbols = Symbols::default();
    let mut out = Collect::default();
    read(
        language,
        ParseRequest {
            path,
            file: FileId(0),
            bytes: ByteSlice(source.as_bytes()),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Wanted,
        },
        &mut Elements::default(),
        &mut symbols,
        &[relation],
        &mut out,
    )?;
    Ok(written(
        out,
        symbols,
        (&[], source.as_bytes()),
        (path, relation),
        language.unknown_reasons(),
    ))
}

/// What a language wrote, read back: `out` and `symbols` are what it left, `files` the path of each file by its number (none for one
/// file), `text` the bytes the spans of its rows are read in, `shown` the path a file cell shows for the one file, and `relation` the
/// relation it writes, and `unknowns` the reasons the language declares for a value it cannot resolve, by their place in its list, so
/// that an unknown cell shows its reason's name. A cause said of one file of a run is shown with that file's path, as the run's finding places it.
#[must_use]
pub fn written(
    out: Collect,
    symbols: Symbols,
    (files, text): (&[&str], &[u8]),
    (shown, relation): (SourcePath<'_>, Relation),
    unknowns: &'static [UnknownName],
) -> Reading {
    let mut causes: Vec<&'static str> = Vec::new();
    for (_, cause, _) in &out.partials {
        if !causes.contains(&cause.as_str()) {
            causes.push(Box::leak(cause.clone().into_boxed_str()));
        }
    }
    for (_, file, cause) in &out.partials_at {
        let at = usize::try_from(file.0)
            .ok()
            .and_then(|place| files.get(place))
            .map_or(String::new(), |path| format!(" (in {path})"));
        let cause = format!("{cause}{at}");
        if !causes.contains(&cause.as_str()) {
            causes.push(Box::leak(cause.into_boxed_str()));
        }
    }
    let read = Read {
        out,
        symbols,
        bytes: text.to_vec(),
        texts: vec![text.to_vec()],
        files: files.iter().map(|each| (*each).to_owned()).collect(),
        unknowns,
    };
    let rows = rows_of_relation(&read, relation, shown.0);
    Reading {
        rows: kept(rows),
        causes: Box::leak(causes.into_boxed_slice()),
    }
}

#[cfg(test)]
#[path = "reading_test.rs"]
mod tests;
