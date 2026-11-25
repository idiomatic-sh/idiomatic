//! The law every language that links both routes keeps: for every catalog relation the language answers, over a source, the rows its
//! old reader (`ParseEngine`) writes and the rows its filler (`Fillers`, through the walk) writes are the same rows. Rows are
//! compared as the rails compare them, rendered and sorted, so the order a relation states is the one the rail already holds it to.

use super::super::service::read;
use super::super::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest, SourceFile};
use super::collect::Collect;
use super::reading::written;
use super::samples::samples_in;
use super::wanted::Want;
use crate::domain::elements::types::Elements;
use crate::domain::facts::service::{base_words, file_words, text};
use crate::domain::facts::types::{CellWord, FactAnswer, FactSource, FileFacts, FileWords, FilledColumns, Wanted};
use crate::domain::fixed_names::service::checked;
use crate::domain::files::types::SourcePath;
use crate::domain::node_table::types::NodeTable;
use crate::domain::symbols::service::intern_derived;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::walk::errors::WalkError;
use crate::inbound::use_cases::walk::types::{FileVisit, RelationNeeds, WalkEngine, WalkRequest};
use ryo_relations::domain::relations::service::{all, kinds};
use crate::domain::elements::service::{end_at, end_of, kind_at, start_at};
use crate::domain::node_table::service::{kind_codes, span_of};
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{CellKind, FileId, Position, Relation};
use ryo_relations::domain::terms::service::{term_text, unknown_reason};
use ryo_relations::domain::terms::types::{NameText, Symbol};
use std::path::Path;

/// The relations of `source` on which the two routes of a language differ, each described with its first differing row; empty
/// when they agree. A language the walk does not read says so as one difference.
///
/// # Examples
///
/// ```
/// use ryo_language::inbound::use_cases::parse::fixtures::routes::route_differences;
/// use ryo_language::inbound::use_cases::walk::use_case::WalkUseCaseImpl;
/// # let _ = (route_differences, WalkUseCaseImpl::new());
/// ```
#[must_use]
pub fn route_differences(
    old: &dyn ParseEngine,
    walk: &dyn WalkEngine,
    path: SourcePath<'_>,
    source: ByteSlice<'_>,
) -> Vec<String> {
    let mut found = Differences { canon: Canon::default(), old, path, source, found: Vec::new() };
    let vocabulary = walk.vocabulary();
    let base = base_words(checked(&vocabulary.fixed.0).expect("walk vocabulary"), &[]);
    let (mut table, mut buffers) = (NodeTable::default(), walk.buffers());
    let request = WalkRequest { path, file: FileId(0), source };
    let mut visit = Compare { found: &mut found, words: file_words(&base) };
    match walk.walk_file(request, RelationNeeds(all()), &mut table, &mut *buffers, &mut visit) {
        Ok(()) => {}
        Err(WalkError::Unsupported) => found.found.push(format!("{}: the walk does not read the file", path.0)),
        Err(error) => found.found.push(format!("{}: the walk failed: {error:?}", path.0)),
    }
    found.found
}

/// The differences over every sample of the language's fixtures folder (`samples_in`): the relation fixtures and the allocation
/// files.
///
/// # Panics
///
/// When a folder of the fixtures cannot be read.
#[must_use]
pub fn fixture_route_differences(old: &dyn ParseEngine, walk: &dyn WalkEngine, fixtures: &Path) -> Vec<String> {
    let samples: Vec<SourceFile> = samples_in(fixtures);
    samples
        .iter()
        .flat_map(|sample| route_differences(old, walk, sample.path, sample.bytes))
        .collect()
}

/// A node as both routes can name it: where it is and what it is. The two routes number a file's nodes in their own ways, so a node
/// cell is compared as this key, numbered by first sight across both routes of one source.
#[derive(Default)]
struct Canon(std::collections::HashMap<(u32, u32, u16), u64>);

impl Canon {
    fn id(&mut self, key: (u32, u32, u16)) -> u64 {
        let next = self.0.len() as u64;
        *self.0.entry(key).or_insert(next)
    }

    /// Replaces each node cell of `collected`'s rows of `relation` by the id of the key `key_of` gives its place.
    fn apply(&mut self, collected: &mut Collect, relation: Relation, key_of: &dyn Fn(u32) -> (u32, u32, u16)) {
        let kinds = kinds(relation);
        for row in collected.words.chunks_mut(kinds.len().max(1)) {
            for (kind, bits) in kinds.iter().zip(row) {
                if *kind == CellKind::Node {
                    *bits = self.id(key_of(u32::try_from(*bits & 0xffff_ffff).unwrap_or(u32::MAX)));
                }
            }
        }
    }
}

struct Differences<'a> {
    canon: Canon,
    old: &'a dyn ParseEngine,
    path: SourcePath<'a>,
    source: ByteSlice<'a>,
    found: Vec<String>,
}

struct Compare<'a, 'b> {
    found: &'a mut Differences<'b>,
    words: FileWords,
}

impl FileVisit for Compare<'_, '_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        let FileFacts { facts, table, .. } = file;
        for &relation in all() {
            self.relation(relation, table, facts);
        }
    }
}

impl Compare<'_, '_> {
    fn relation(&mut self, relation: Relation, table: &NodeTable, facts: &mut dyn FactSource) {
        let (path, source) = (self.found.path, self.found.source);
        let mut columns = FilledColumns::default();
        let answer = facts.fill(relation, Wanted::Whole, &mut self.words, &mut columns);
        let old = old_reading(self.found.old, &mut self.found.canon, path, source, relation);
        let shown = format!("{}: {relation:?}", path.0);
        let (mut symbols, mut collected) = (Symbols::default(), Collect::default());
        match answer {
            // The four relations the node table holds are compared as trees by the walk contract's own test.
            FactAnswer::Refused(AbsenceCause::NoSuchConstruct) => return,
            FactAnswer::Refused(cause) => {
                // A relation the node table holds, or one the language says it does not have: nothing to compare when the old route
                // says the same.
                if self.found.old.absence_of(relation) != Some(cause) && old.is_ok() && has_rows(&old) {
                    self.found.found.push(format!("{shown}: the filler refuses ({cause:?}) and the old reader writes rows"));
                }
                return;
            }
            FactAnswer::Partial(partial) => collected.partials.push((relation, partial.cause.0.to_string(), partial.missing)),
            FactAnswer::Answered(_) => {}
        }
        let rows = columns.0.first().map_or(0, |column| column.0.len());
        for row in 0..rows {
            collected.relations.push(relation);
            for (kind, column) in kinds(relation).iter().zip(&columns.0) {
                let bits = column.0[row].0;
                collected.words.push(if *kind == CellKind::Symbol { symbol_bits(&self.words, &mut symbols, source, bits) } else { bits });
            }
        }
        let (kind_codes, spans) = (kind_codes(table), table);
        self.found.canon.apply(&mut collected, relation, &|at| {
            if at as usize >= kind_codes.len() {
                return (u32::MAX, u32::MAX, u16::MAX);
            }
            let span = span_of(spans, Position(at));
            (span.start.0, span.end.0, kind_codes.get(at as usize).copied().unwrap_or(u16::MAX))
        });
        let new = written(collected, symbols, (&[], source.0), (path, relation), self.found.old.unknown_reasons());
        let Ok(old) = old else {
            self.found.found.push(format!("{shown}: the old reader refuses the file"));
            return;
        };
        if let Some(first) = first_difference(old.rows, new.rows) {
            self.found.found.push(format!("{shown}: {first}"));
        }
        if old.causes != new.causes {
            self.found.found.push(format!("{shown}: partial causes differ: old {:?}, fill {:?}", old.causes, new.causes));
        }
    }
}

fn has_rows(old: &Result<super::reading::Reading, super::super::errors::ParseError>) -> bool {
    old.as_ref().is_ok_and(|reading| !reading.rows.is_empty())
}

/// What the old reader wrote of `relation`, as `reading` reads it, its node cells named by `canon`.
fn old_reading(
    old: &dyn ParseEngine,
    canon: &mut Canon,
    path: SourcePath<'_>,
    source: ByteSlice<'_>,
    relation: Relation,
) -> Result<super::reading::Reading, super::super::errors::ParseError> {
    let mut symbols = Symbols::default();
    let mut out = Collect::default();
    let mut elements = Elements::default();
    read(
        old,
        ParseRequest { path, file: FileId(0), bytes: source, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted },
        &mut elements,
        &mut symbols,
        &[relation],
        &mut out,
    )?;
    canon.apply(&mut out, relation, &|at| {
        let place = Position(at);
        if place >= end_of(&elements) {
            return (u32::MAX, u32::MAX, u16::MAX);
        }
        (start_at(&elements, place).0, end_at(&elements, place).0, kind_at(&elements, place).0)
    });
    Ok(written(out, symbols, (&[], source.0), (path, relation), old.unknown_reasons()))
}

/// A text cell of a fill as the old route's symbols hold it: the word's text interned; a marker of the catalog (an unknown
/// reason, no symbol) is kept as it is.
fn symbol_bits(words: &FileWords, symbols: &mut Symbols, source: ByteSlice<'_>, bits: u64) -> u64 {
    let symbol = Symbol(u32::try_from(bits).unwrap_or(u32::MAX));
    if bits >= u64::from(u32::MAX) || term_text(symbol).is_some() || unknown_reason(symbol).is_some() {
        return bits;
    }
    let NameText(spelled) = text(words, CellWord(bits));
    u64::from(intern_derived(symbols, source, NameText(spelled)).0)
}

/// The rows only one route wrote, up to three of each, and the counts.
fn first_difference(old: &[&[Want]], new: &[&[Want]]) -> Option<String> {
    let mut held: std::collections::HashMap<String, i64> = std::collections::HashMap::new();
    for row in old {
        *held.entry(format!("{row:?}")).or_default() += 1;
    }
    let mut only_new = Vec::new();
    for row in new {
        let shown = format!("{row:?}");
        match held.get_mut(&shown) {
            Some(count) if *count > 0 => *count -= 1,
            _ => only_new.push(shown),
        }
    }
    let only_old: Vec<_> = held.into_iter().flat_map(|(row, count)| std::iter::repeat_n(row, usize::try_from(count).unwrap_or(0))).collect();
    if only_old.is_empty() && only_new.is_empty() {
        return None;
    }
    Some(format!(
        "{} old rows, {} filled; {} only in old, e.g. {:?}; {} only in fill, e.g. {:?}",
        old.len(),
        new.len(),
        only_old.len(),
        &only_old[..only_old.len().min(3)],
        only_new.len(),
        &only_new[..only_new.len().min(3)]
    ))
}
