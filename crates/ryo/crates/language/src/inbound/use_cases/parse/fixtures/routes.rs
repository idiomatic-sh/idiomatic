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
use crate::domain::node_table::service::{field_of, kind_codes, ordinal_of, own_end, parent_of, span_of};
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
    route_counted(old, walk, path, source).2
}

/// `route_differences`, with how many relations were compared and how many rows the fillers wrote.
fn route_counted(
    old: &dyn ParseEngine,
    walk: &dyn WalkEngine,
    path: SourcePath<'_>,
    source: ByteSlice<'_>,
) -> (usize, usize, Vec<String>) {
    let mut found = Differences { old_completeness: old_completeness(old, path, source), relations: 0, rows: 0, canon: Canon::default(), old, path, source, found: Vec::new() };
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
    let mut found = found;
    found.found.extend(adapter_view_differences(old, walk, path, source));
    (found.relations, found.rows, found.found)
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
    let mut found = metadata_differences(old, walk);
    for sample in &samples {
        let request = ParseRequest {
            path: sample.path,
            file: FileId(0),
            bytes: sample.bytes,
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Wanted,
        };
        if old.recognition(&request) != walk.recognizes(sample.path, false) {
            found.push(format!("{}: recognition differs", sample.path.0));
        }
        found.extend(route_differences(old, walk, sample.path, sample.bytes));
    }
    found
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
    /// The completeness the old route reports for the file (syntax, lexical): what the walk's `completeness` must equal.
    old_completeness: Option<(crate::domain::diagnostics::types::SyntaxCompleteness, crate::domain::diagnostics::types::SyntaxCompleteness)>,
    relations: usize,
    rows: usize,
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
        let walked = facts.completeness();
        if let Some(old) = self.found.old_completeness
            && old != (walked.syntax, walked.lexical)
        {
            let shown = self.found.path.0;
            self.found.found.push(format!("{shown}: completeness differs: old (syntax, lexical) {old:?}, walk {:?}", (walked.syntax, walked.lexical)));
        }
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
        self.found.relations += 1;
        self.found.rows += new.rows.len();
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

/// What the language states about itself, by the old route and by the walk: availability and expansions of every relation, the
/// skipped directories, the configuration, the capabilities and the unknown reasons.
#[must_use]
pub fn metadata_differences(old: &dyn ParseEngine, walk: &dyn WalkEngine) -> Vec<String> {
    let mut found = Vec::new();
    for &relation in all() {
        if old.absence_of(relation) != walk.absence_of(relation) {
            found.push(format!("{relation:?}: availability differs: old {:?}, walk {:?}", old.absence_of(relation), walk.absence_of(relation)));
        }
        if old.absence_of(relation).is_none() && old.includes_expansions(relation) != walk.includes_expansions(relation) {
            found.push(format!("{relation:?}: expansions differ: old {}, walk {}", old.includes_expansions(relation), walk.includes_expansions(relation)));
        }
    }
    if old.skips() != walk.skips() {
        found.push("skipped directories differ".to_owned());
    }
    if old.configuration_identity() != walk.configuration_identity() {
        found.push("configuration identity differs".to_owned());
    }
    if old.capabilities() != walk.capabilities() {
        found.push(format!("capabilities differ: old {:?}, walk {:?}", old.capabilities(), walk.capabilities()));
    }
    if old.unknown_reasons() != walk.unknown_reasons() {
        found.push("unknown reasons differ".to_owned());
    }
    found
}

/// What a comparison over whole directories found.
#[derive(Debug, Default)]
pub struct CorpusReport {
    /// The files of the language found under the roots.
    pub files: usize,
    /// The relations compared, over all files.
    pub relations: usize,
    /// The rows the fillers wrote, over all files.
    pub rows: usize,
    /// The files on which the routes differ.
    pub differing: usize,
    /// The differences, each with its file and first differing row.
    pub differences: Vec<String>,
}

/// The two routes over every file the language recognises under `roots`, at any depth, not entering the language's skipped
/// directories or `.git`: rows and recognition, old reader against filler, file by file; the metadata once. A file that is not text
/// the language reads is skipped, never counted.
///
/// # Panics
///
/// When a root cannot be read.
#[must_use]
pub fn corpus_route_report(old: &dyn ParseEngine, walk: &dyn WalkEngine, roots: &[std::path::PathBuf]) -> CorpusReport {
    let mut report = CorpusReport { differences: metadata_differences(old, walk), ..CorpusReport::default() };
    let skipped: Vec<&str> = old.skips().iter().map(|each| each.0).collect();
    let mut stack: Vec<(std::path::PathBuf, std::path::PathBuf)> = roots.iter().map(|root| (root.clone(), root.clone())).collect();
    while let Some((root, path)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            assert!(path != root, "a corpus root cannot be read: {}", path.display());
            continue;
        };
        let mut entries: Vec<_> = entries.filter_map(Result::ok).map(|entry| entry.path()).collect();
        entries.sort();
        for entry in entries {
            let name = entry.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_owned();
            if entry.is_dir() {
                if name != ".git" && !skipped.contains(&name.as_str()) {
                    stack.push((root.clone(), entry));
                }
                continue;
            }
            let Ok(bytes) = std::fs::read(&entry) else {
                continue;
            };
            let shown = entry.strip_prefix(&root).unwrap_or(&entry).to_string_lossy().replace('\\', "/");
            let request = ParseRequest {
                path: SourcePath(&shown),
                file: FileId(0),
                bytes: ByteSlice(&bytes),
                notes: NoteRequest::Unwanted,
                fields: FieldRequest::Wanted,
            };
            if old.recognition(&request) != super::super::types::Recognition::Reads {
                continue;
            }
            report.files += 1;
            if walk.recognizes(SourcePath(&shown), false) != super::super::types::Recognition::Reads {
                report.differing += 1;
                report.differences.push(format!("{shown}: recognition differs"));
                continue;
            }
            let (relations, rows, found) = route_counted(old, walk, SourcePath(&shown), ByteSlice(&bytes));
            report.relations += relations;
            report.rows += rows;
            if !found.is_empty() {
                report.differing += 1;
                report.differences.extend(found);
            }
        }
    }
    report
}

/// What the engine reads through the numbered visit (`NumberedWalk`, the upper layer), against what the old route gives it today:
/// the rows of the four node-table relations (`syntax-tree` with its text, `syntax-child` with its field names, `semantic-syntax`
/// and `semantic-child`), the token word of every position, and the completeness the visit reports. A TOKEN'S TEXT IS THE RAW
/// WRITTEN TEXT, as the old `syntax-tree` gives it: a decoded value belongs to the relations that state it. Nothing is compared for
/// a language whose old route has no numbered visit yet.
#[must_use]
pub fn adapter_view_differences(old: &dyn ParseEngine, walk: &dyn WalkEngine, path: SourcePath<'_>, source: ByteSlice<'_>) -> Vec<String> {
    let Some(numbered) = old.file_relations().and_then(|relations| relations.numbered()) else {
        return Vec::new();
    };
    let kind_names = walk.vocabulary().kinds;
    let mut found = Vec::new();
    let mut canon = Canon::default();
    let request = ParseRequest { path, file: FileId(0), bytes: source, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
    // The old route: its completeness and the rows of the four relations.
    let mut elements = Elements::default();
    let mut sink = Collect::default();
    if read(old, request, &mut elements, &mut Symbols::default(), &[Relation::SyntaxTree], &mut sink).is_err() {
        return vec![format!("{}: the old reader refuses the file", path.0)];
    }
    let old_completeness = (
        crate::domain::elements::service::has_incomplete_syntax(&elements),
        crate::domain::elements::service::has_incomplete_lexical_stream(&elements),
    );
    let relations = [Relation::SyntaxTree, Relation::SyntaxChild, Relation::SemanticSyntax, Relation::SemanticChild];
    let old_rows: Vec<_> = relations.iter().map(|relation| old_reading(old, &mut canon, path, source, *relation)).collect();
    // The numbered visit: what the engine gets from it.
    let mut out = Collect::default();
    let mut symbols = Symbols::default();
    let mut completeness = None;
    let mut reader = numbered.open();
    let visited = reader.visit(request, all(), &[], None, &mut symbols, &mut |file, symbols| {
        completeness = Some((file.syntax, file.lexical));
        let table = file.tree;
        let count = kind_codes(table).len();
        // `syntax-tree` and `syntax-child` are the file's own tree: the embedded segments are no part of them.
        let own = usize::try_from(own_end(table).0).unwrap_or(count).min(count);
        let key = |at: u32| -> (u32, u32, u16) {
            if at as usize >= count {
                return (u32::MAX, u32::MAX, u16::MAX);
            }
            let span = span_of(table, Position(at));
            (span.start.0, span.end.0, kind_codes(table)[at as usize])
        };
        for at in 0..own {
            let position = Position(u32::try_from(at).unwrap_or(u32::MAX));
            let name = kind_names.0.get(usize::from(kind_codes(table)[at])).copied().unwrap_or(NameText(""));
            let span = span_of(table, position);
            let id = canon.id(key(position.0));
            out.relations.push(Relation::SyntaxTree);
            out.words.extend([
                id,
                u64::from(intern_derived(symbols, source, name).0),
                u64::from(file.words.get(at).map_or(ryo_relations::domain::relations::constants::NO_SYMBOL, |word| *word).0),
                0,
                (u64::from(span.start.0) << 32) | u64::from(span.end.0),
            ]);
            let parent = parent_of(table, position);
            if parent != position {
                let field = file.fields.0.get(usize::from(field_of(table, position).0)).copied().unwrap_or(NameText(""));
                out.relations.push(Relation::SyntaxChild);
                out.words.extend([
                    canon.id(key(parent.0)),
                    id,
                    u64::from(ordinal_of(table, position).0),
                    u64::from(intern_derived(symbols, source, field).0),
                    0,
                ]);
            }
        }
        let mut semantic = Collect::default();
        let _ = file.facts.project_structure(&[Relation::SemanticSyntax, Relation::SemanticChild], symbols, &mut semantic);
        for relation in [Relation::SemanticSyntax, Relation::SemanticChild] {
            let mut rest = semantic.words.as_slice();
            for each in &semantic.relations {
                let (row, after) = rest.split_at(kinds(*each).len());
                rest = after;
                if *each == relation {
                    let mut row = row.to_vec();
                    canon_nodes(&mut canon, relation, &mut row, &key);
                    out.relations.push(relation);
                    out.words.extend(row);
                }
            }
        }
    });
    if visited.is_err() {
        found.push(format!("{}: the numbered visit refuses the file", path.0));
        return found;
    }
    if let Some((syntax, lexical)) = completeness
        && (syntax, lexical) != old_completeness
    {
        found.push(format!("{}: completeness differs: old {old_completeness:?}, numbered {:?}", path.0, (syntax, lexical)));
    }
    let read = super::rows::Read {
        out,
        symbols,
        bytes: source.0.to_vec(),
        texts: vec![source.0.to_vec()],
        files: Vec::new(),
        unknowns: old.unknown_reasons(),
    };
    for (relation, old_rows) in relations.iter().zip(old_rows) {
        let new_rows = super::wanted::kept(super::rows::rows_of_relation(&read, *relation, path.0));
        match old_rows {
            Ok(old) => {
                if let Some(first) = first_difference(old.rows, new_rows) {
                    found.push(format!("{}: the numbered view of {relation:?}: {first}", path.0));
                }
            }
            Err(_) => found.push(format!("{}: the old reader refuses {relation:?}", path.0)),
        }
    }
    found
}

/// Names the node cells of a row by the key of their place.
fn canon_nodes(canon: &mut Canon, relation: Relation, row: &mut [u64], key: &dyn Fn(u32) -> (u32, u32, u16)) {
    for (kind, bits) in kinds(relation).iter().zip(row) {
        if *kind == CellKind::Node {
            *bits = canon.id(key(u32::try_from(*bits & 0xffff_ffff).unwrap_or(u32::MAX)));
        }
    }
}

/// What the old route reports of the file's completeness: the syntax and the lexical stream, each in full or in part, as the
/// diagnostics the file carries say; none when the old reader refuses the file.
fn old_completeness(
    old: &dyn ParseEngine,
    path: SourcePath<'_>,
    source: ByteSlice<'_>,
) -> Option<(crate::domain::diagnostics::types::SyntaxCompleteness, crate::domain::diagnostics::types::SyntaxCompleteness)> {
    let request = ParseRequest { path, file: FileId(0), bytes: source, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
    let mut elements = Elements::default();
    read(old, request, &mut elements, &mut Symbols::default(), &[Relation::SyntaxError], &mut Collect::default()).ok()?;
    Some((
        crate::domain::elements::service::has_incomplete_syntax(&elements),
        crate::domain::elements::service::has_incomplete_lexical_stream(&elements),
    ))
}
