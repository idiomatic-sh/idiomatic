//! The law every language that links both routes keeps: for every catalog relation the language answers, over a source, the rows its
//! old reader (`ParseEngine`) writes and the rows its filler (`Fillers`, through the walk) writes are the same rows. Rows are
//! compared as the rails compare them, rendered and sorted, so the order a relation states is the one the rail already holds it to.

use super::super::service::read;
use super::super::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest, SourceFile};
use super::collect::Collect;
use super::reading::written;
use super::samples::{samples_in, spelled_in};
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
    route_counted(old, walk, path, source, &mut Adjudication::default()).2
}

/// `route_differences`, with how many relations were compared and how many rows the fillers wrote.
fn route_counted(
    old: &dyn ParseEngine,
    walk: &dyn WalkEngine,
    path: SourcePath<'_>,
    source: ByteSlice<'_>,
    adjudication: &mut Adjudication,
) -> (usize, usize, Vec<String>) {
    adjudication.visit(path.0);
    let mut found = Differences { old_completeness: old_completeness(old, path, source), relations: 0, rows: 0, canon: Canon::default(), old, path, source, found: Vec::new(), complete: None, adjudication };
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
    fixture_routes(old, walk, fixtures).0
}

/// `fixture_route_differences`, and how many differing pairs `adjudicated_routes.tsv` of the fixtures judged right on one side.
///
/// # Panics
///
/// When a folder of the fixtures cannot be read, or the adjudication list is malformed.
#[must_use]
pub fn fixture_routes(old: &dyn ParseEngine, walk: &dyn WalkEngine, fixtures: &Path) -> (Vec<String>, usize) {
    let samples: Vec<SourceFile> = samples_in(fixtures).into_iter().chain(spelled_in(fixtures)).collect();
    let mut adjudication = Adjudication::load(fixtures).expect("the adjudication list of the fixtures is well formed");
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
        found.extend(route_counted(old, walk, sample.path, sample.bytes, &mut adjudication).2);
    }
    found.extend(adjudication.stale());
    (found, adjudication.adjudicated)
}

/// A node as both routes can name it: where it is and what it is. The two routes number a file's nodes in their own ways, so a node
/// cell is compared as this key, numbered by first sight across both routes of one source. An embedded segment's nodes are keyed by
/// the file offsets they cover (both routes give the same), never by the order the segment was embedded in.
///
/// A column that holds a CLASS REPRESENTATIVE ("an opaque real syntax representative in the same file": `ExpressionShape.shape`,
/// `ConstantKey.key`) names whichever member of its class a route met first, and the routes meet embedded segments in different
/// orders (the old writer embeds some while the table is written and some in a later pass). Such a cell is compared as the least key
/// among the members of its class (the rows that carry it), which does not depend on any numbering.
#[derive(Default)]
pub(super) struct Canon(std::collections::HashMap<(u32, u32, u16), u64>, bool);

/// The column that holds a class representative in the rows of `relation`, the first column holding the member.
const fn representative_column(relation: Relation) -> Option<usize> {
    match relation {
        Relation::ExpressionShape | Relation::ConstantKey => Some(1),
        _ => None,
    }
}

impl Canon {
    /// A numbering that names a node by a hash of its key, so that the name does not depend on the order the rows are met in:
    /// what a frozen record needs.
    pub(super) fn hashed() -> Self {
        Self(std::collections::HashMap::new(), true)
    }

    pub(super) fn id(&mut self, key: (u32, u32, u16)) -> u64 {
        if self.1 {
            return u64::from(super::frozen::key_hash(key));
        }
        let next = self.0.len() as u64;
        *self.0.entry(key).or_insert(next)
    }

    /// `apply` over rows of different relations written one after another, as a read that wants several relations writes them.
    pub(super) fn apply_mixed(&mut self, collected: &mut Collect, key_of: &dyn Fn(u32) -> (u32, u32, u16)) {
        let node = |bits: u64| u32::try_from(bits & 0xffff_ffff).unwrap_or(u32::MAX);
        let mut least: std::collections::HashMap<(Relation, u32), (u32, u32, u16)> = std::collections::HashMap::new();
        let mut at = 0;
        for relation in &collected.relations {
            let kinds = kinds(*relation);
            if let Some(column) = representative_column(*relation).filter(|column| *column < kinds.len() && kinds[*column] == CellKind::Node) {
                let row = &collected.words[at..at + kinds.len()];
                let key = key_of(node(row[0]));
                least.entry((*relation, node(row[column]))).and_modify(|held| *held = (*held).min(key)).or_insert(key);
            }
            at += kinds.len();
        }
        let mut at = 0;
        for relation in &collected.relations {
            let kinds = kinds(*relation);
            let representative = representative_column(*relation);
            for (column, kind) in kinds.iter().enumerate() {
                if *kind == CellKind::Node {
                    let bits = collected.words[at + column];
                    let key = if representative == Some(column) { least.get(&(*relation, node(bits))).copied().unwrap_or_else(|| key_of(node(bits))) } else { key_of(node(bits)) };
                    collected.words[at + column] = self.id(key);
                }
            }
            at += kinds.len();
        }
    }

    /// Replaces each node cell of `collected`'s rows of `relation` by the id of the key `key_of` gives its place; a class
    /// representative by the id of the least key among its class.
    pub(super) fn apply(&mut self, collected: &mut Collect, relation: Relation, key_of: &dyn Fn(u32) -> (u32, u32, u16)) {
        let kinds = kinds(relation);
        let node = |bits: u64| u32::try_from(bits & 0xffff_ffff).unwrap_or(u32::MAX);
        let representative = representative_column(relation).filter(|at| *at < kinds.len() && kinds[*at] == CellKind::Node);
        let mut least: std::collections::HashMap<u32, (u32, u32, u16)> = std::collections::HashMap::new();
        if let Some(at) = representative {
            for row in collected.words.chunks(kinds.len().max(1)) {
                let key = key_of(node(row[0]));
                least.entry(node(row[at])).and_modify(|held| *held = (*held).min(key)).or_insert(key);
            }
        }
        for row in collected.words.chunks_mut(kinds.len().max(1)) {
            for (column, (kind, bits)) in kinds.iter().zip(row).enumerate() {
                if *kind == CellKind::Node {
                    let key = if representative == Some(column) { least.get(&node(*bits)).copied().unwrap_or_else(|| key_of(node(*bits))) } else { key_of(node(*bits)) };
                    *bits = self.id(key);
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
    /// The old route read once with every relation wanted, for a language whose completion needs a prepared model: `None` until the
    /// first relation asks, then `Some(None)` when the old reader refuses the file.
    complete: Option<Option<CompleteOld>>,
    /// The differences judged once and for all (`adjudicated_routes.tsv`), and which of them this run met.
    adjudication: &'a mut Adjudication,
}

impl Differences<'_> {
    /// What the old route wrote of `relation`. A language whose completion needs a prepared model (`PreparedModel`) writes some of
    /// its rows only when a reader of what completion adds is wanted, so its old route is read the way a request reads it: ONCE per
    /// file with every relation wanted, and each relation's rows are taken from that read. Any other language is read one relation
    /// at a time, as before.
    fn old_rows(&mut self, relation: Relation) -> Result<super::reading::Reading, super::super::errors::ParseError> {
        old_rows_of(self.old, &mut self.canon, self.path, self.source, &mut self.complete, relation)
    }
}

/// What the old route wrote of `relation` of one file, as `Differences::old_rows` reads it: shared with the frozen record, which
/// must read the old route the same way. `complete` holds the one read of a language whose completion needs a prepared model.
pub(super) fn old_rows_of(
    old: &dyn ParseEngine,
    canon: &mut Canon,
    path: SourcePath<'_>,
    source: ByteSlice<'_>,
    complete: &mut Option<Option<CompleteOld>>,
    relation: Relation,
) -> Result<super::reading::Reading, super::super::errors::ParseError> {
    if old.completion_requirement() != crate::domain::elements::types::CompletionRequirement::PreparedModel {
        return old_reading(old, canon, path, source, relation);
    }
    if complete.is_none() {
        *complete = Some(old_complete(old, canon, path, source));
    }
    match complete.as_ref().and_then(Option::as_ref) {
        Some(read) => Ok(read.reading(relation, path)),
        None => old_reading(old, canon, path, source, relation),
    }
}

/// The old route's rows of every relation of one file, read together, their node cells already named.
pub(super) struct CompleteOld(super::rows::Read);

impl CompleteOld {
    /// The rows and the partial causes of `relation`, as `written` gives them for a read of that relation alone.
    fn reading(&self, relation: Relation, shown: SourcePath<'_>) -> super::reading::Reading {
        let mut causes: Vec<&'static str> = Vec::new();
        for (each, cause, _) in &self.0.out.partials {
            if *each == relation && !causes.contains(&cause.as_str()) {
                causes.push(Box::leak(cause.clone().into_boxed_str()));
            }
        }
        for (each, _, cause) in &self.0.out.partials_at {
            if *each == relation && !causes.contains(&cause.as_str()) {
                causes.push(Box::leak(cause.clone().into_boxed_str()));
            }
        }
        super::reading::Reading {
            rows: super::wanted::kept(super::rows::rows_of_relation(&self.0, relation, shown.0)),
            causes: Box::leak(causes.into_boxed_slice()),
        }
    }
}

/// Reads the old route of `path` once, wanting every relation, as a request that reads what completion adds does.
fn old_complete(old: &dyn ParseEngine, canon: &mut Canon, path: SourcePath<'_>, source: ByteSlice<'_>) -> Option<CompleteOld> {
    let (mut symbols, mut out, mut elements) = (Symbols::default(), Collect::default(), Elements::default());
    let request = ParseRequest { path, file: FileId(0), bytes: source, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
    read(old, request, &mut elements, &mut symbols, all(), &mut out).ok()?;
    canon.apply_mixed(&mut out, &|at| {
        let place = Position(at);
        if place >= end_of(&elements) {
            return (u32::MAX, u32::MAX, u16::MAX);
        }
        (start_at(&elements, place).0, end_at(&elements, place).0, kind_at(&elements, place).0)
    });
    Some(CompleteOld(super::rows::Read { out, symbols, bytes: source.0.to_vec(), texts: vec![source.0.to_vec()], files: Vec::new(), unknowns: old.unknown_reasons() }))
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
        let old = self.found.old_rows(relation);
        let shown = format!("{}: {relation:?}", path.0);
        let new = match walked(relation, table, facts, &mut self.words, (path, source), self.found.old.unknown_reasons(), &mut self.found.canon) {
            // The four relations the node table holds are compared as trees by the walk contract's own test.
            Walked::Skipped => return,
            Walked::Refused(cause) => {
                // A relation the node table holds, or one the language says it does not have: nothing to compare when the old route
                // says the same.
                if self.found.old.absence_of(relation) != Some(cause) && old.is_ok() && has_rows(&old) {
                    self.found.found.push(format!("{shown}: the filler refuses ({cause:?}) and the old reader writes rows"));
                }
                return;
            }
            Walked::Rows(new) => new,
        };
        self.found.relations += 1;
        self.found.rows += new.rows.len();
        let Ok(old) = old else {
            self.found.found.push(format!("{shown}: the old reader refuses the file"));
            return;
        };
        if let Some((rows, first)) = first_difference(old.rows, new.rows) {
            let (path, name) = (path.0, format!("{relation:?}"));
            if let Some(difference) = self.found.adjudication.settle(path, &name, rows, format!("{shown}: {first}")) {
                self.found.found.push(difference);
            }
        }
        if old.causes != new.causes {
            self.found.found.push(format!("{shown}: partial causes differ: old {:?}, fill {:?}", old.causes, new.causes));
        }
    }
}

/// What the walk's filler of a relation gave for a file.
pub(super) enum Walked {
    /// A relation the node table holds, which no filler writes.
    Skipped,
    /// The language does not have the relation, or the node table holds it.
    Refused(AbsenceCause),
    /// The rows, each in full, and the causes for which they are read in part.
    Rows(super::reading::Reading),
}

/// What the filler of `relation` writes for the walked file, its node cells named by `canon`.
pub(super) fn walked(
    relation: Relation,
    table: &NodeTable,
    facts: &mut dyn FactSource,
    words: &mut FileWords,
    (path, source): (SourcePath<'_>, ByteSlice<'_>),
    unknowns: &'static [ryo_relations::domain::terms::types::UnknownName],
    canon: &mut Canon,
) -> Walked {
    let mut columns = FilledColumns::default();
    let answer = facts.fill(relation, Wanted::Whole, words, &mut columns);
    let (mut symbols, mut collected) = (Symbols::default(), Collect::default());
    match answer {
        FactAnswer::Refused(AbsenceCause::NoSuchConstruct) => return Walked::Skipped,
        FactAnswer::Refused(cause) => return Walked::Refused(cause),
        FactAnswer::Partial(partial) => collected.partials.push((relation, partial.cause.0.to_string(), partial.missing)),
        FactAnswer::Answered(_) => {}
    }
    let rows = columns.0.first().map_or(0, |column| column.0.len());
    for row in 0..rows {
        collected.relations.push(relation);
        for (kind, column) in kinds(relation).iter().zip(&columns.0) {
            let bits = column.0[row].0;
            collected.words.push(if *kind == CellKind::Symbol { symbol_bits(words, &mut symbols, source, bits) } else { bits });
        }
    }
    let (kind_codes, spans) = (kind_codes(table), table);
    canon.apply(&mut collected, relation, &|at| {
        if at as usize >= kind_codes.len() {
            return (u32::MAX, u32::MAX, u16::MAX);
        }
        let span = span_of(spans, Position(at));
        (span.start.0, span.end.0, kind_codes.get(at as usize).copied().unwrap_or(u16::MAX))
    });
    Walked::Rows(written(collected, symbols, (&[], source.0), (path, relation), unknowns))
}

fn has_rows(old: &Result<super::reading::Reading, super::super::errors::ParseError>) -> bool {
    old.as_ref().is_ok_and(|reading| !reading.rows.is_empty())
}

/// What the old reader wrote of `relation`, as `reading` reads it, its node cells named by `canon`.
pub(super) fn old_reading(
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
fn first_difference(old: &[&[Want]], new: &[&[Want]]) -> Option<(usize, String)> {
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
    Some((
        only_old.len() + only_new.len(),
        format!(
            "{} old rows, {} filled; {} only in old, e.g. {:?}; {} only in fill, e.g. {:?}",
            old.len(),
            new.len(),
            only_old.len(),
            &only_old[..only_old.len().min(3)],
            only_new.len(),
            &only_new[..only_new.len().min(3)]
        ),
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
    /// How many differing pairs the adjudication list judged right on one side (not in `differences`).
    pub adjudicated: usize,
}

/// The two routes over every file the language recognises under `roots`, at any depth, not entering the language's skipped
/// directories or `.git`: rows and recognition, old reader against filler, file by file; the metadata once. A file that is not text
/// the language reads is skipped, never counted.
///
/// # Panics
///
/// When a root cannot be read.
#[must_use]
pub fn corpus_route_report(old: &dyn ParseEngine, walk: &dyn WalkEngine, roots: &[std::path::PathBuf], adjudication: &mut Adjudication) -> CorpusReport {
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
            let (relations, rows, found) = route_counted(old, walk, SourcePath(&shown), ByteSlice(&bytes), adjudication);
            report.relations += relations;
            report.rows += rows;
            if !found.is_empty() {
                report.differing += 1;
                report.differences.extend(found);
            }
        }
    }
    report.differences.extend(adjudication.stale());
    report.adjudicated = adjudication.adjudicated;
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
                if let Some((_, first)) = first_difference(old.rows, new_rows) {
                    found.push(format!("{}: the numbered view of {relation:?}: {first}", path.0));
                }
            }
            Err(_) => found.push(format!("{}: the old reader refuses {relation:?}", path.0)),
        }
    }
    found
}

/// Names the node cells of a row by the key of their place.
pub(super) fn canon_nodes(canon: &mut Canon, relation: Relation, row: &mut [u64], key: &dyn Fn(u32) -> (u32, u32, u16)) {
    for (kind, bits) in kinds(relation).iter().zip(row) {
        if *kind == CellKind::Node {
            *bits = canon.id(key(u32::try_from(*bits & 0xffff_ffff).unwrap_or(u32::MAX)));
        }
    }
}

/// What the old route reports of the file's completeness: the syntax and the lexical stream, each in full or in part, as the
/// diagnostics the file carries say; none when the old reader refuses the file.
pub(super) fn old_completeness(
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

/// Which kinds a language spells and which of them its fixtures show with text. A kind a provider declares spelled (`spells`) but that
/// no fixture ever reads with a non-empty word on a route is a kind whose text rule nothing checks: the leaf rule once starved every
/// Rust token because no fixture held a spelled identifier where it mattered, and the comparison read zero.
#[derive(Debug, Default)]
pub struct Coverage {
    /// The kinds `spells` says yes to, by name.
    pub spelled: std::collections::BTreeSet<String>,
    /// The kinds that occur at all in the fixtures (any position, text or none).
    pub occurring: std::collections::BTreeSet<String>,
    /// The kinds seen with a non-empty word in the old reader's elements.
    pub text_old: std::collections::BTreeSet<String>,
    /// The kinds seen with a non-empty word through the numbered visit; empty, and `numbered` false, for a language with none.
    pub text_walk: std::collections::BTreeSet<String>,
    /// Whether the language has a numbered visit (the walk route's words).
    pub numbered: bool,
}

impl Coverage {
    /// The spelled kinds not seen with text on every route the language has: the uncovered ones.
    #[must_use]
    pub fn uncovered(&self) -> Vec<&str> {
        self.spelled
            .iter()
            .filter(|kind| !self.text_old.contains(*kind) || (self.numbered && !self.text_walk.contains(*kind)))
            .map(String::as_str)
            .collect()
    }

    /// The spelled kinds that occur in no fixture at all.
    #[must_use]
    pub fn never_occurring(&self) -> Vec<&str> {
        self.spelled.iter().filter(|kind| !self.occurring.contains(*kind)).map(String::as_str).collect()
    }
}

/// The coverage of a language's spelled kinds over the samples of `fixtures`, on the old reader's elements and the numbered visit.
///
/// # Panics
///
/// When a folder of the fixtures cannot be read.
#[must_use]
pub fn coverage(old: &dyn ParseEngine, walk: &dyn WalkEngine, fixtures: &Path) -> Coverage {
    let names = walk.vocabulary().kinds;
    let name_of = |code: u16| names.0.get(usize::from(code)).map_or_else(|| format!("#{code}"), |name| name.0.to_owned());
    let mut found = Coverage::default();
    for code in 0..names.0.len() {
        let code = u16::try_from(code).unwrap_or(u16::MAX);
        if walk.spells(ryo_relations::domain::relations::types::KindBits(code)) {
            found.spelled.insert(name_of(code));
        }
    }
    let numbered = old.file_relations().and_then(|relations| relations.numbered());
    found.numbered = numbered.is_some();
    for sample in samples_in(fixtures).into_iter().chain(spelled_in(fixtures)) {
        let request =
            ParseRequest { path: sample.path, file: FileId(0), bytes: sample.bytes, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
        let mut elements = Elements::default();
        if read(old, request, &mut elements, &mut Symbols::default(), &[Relation::SyntaxTree], &mut Collect::default()).is_ok() {
            for at in 0..end_of(&elements).0 {
                let place = Position(at);
                let name = name_of(kind_at(&elements, place).0);
                if crate::domain::elements::service::text_at(&elements, place) != ryo_relations::domain::relations::constants::NO_SYMBOL {
                    found.text_old.insert(name.clone());
                }
                found.occurring.insert(name);
            }
        }
        if let Some(numbered) = &numbered {
            let mut symbols = Symbols::default();
            let mut reader = numbered.open();
            let _ = reader.visit(request, all(), &[], None, &mut symbols, &mut |file, _| {
                for (at, word) in file.words.iter().enumerate() {
                    if *word != ryo_relations::domain::relations::constants::NO_SYMBOL
                        && let Some(code) = kind_codes(file.tree).get(at)
                    {
                        found.text_walk.insert(name_of(*code));
                    }
                }
            });
        }
    }
    found
}

/// The reviewed list of spelled kinds no source can show with text, from the language's `unreachable_kinds.tsv` (one line each:
/// the kind, a tab, the reason). The check fails on a spelled kind that is neither covered nor listed here, on a listed kind that is
/// covered after all or not spelled at all (a stale entry), and on an entry without a reason.
pub const UNREACHABLE_KINDS: &str = "unreachable_kinds.tsv";

/// What is wrong with the coverage of a language given its exemption list: empty when every spelled kind is covered or listed.
#[must_use]
pub fn coverage_failures(covered: &Coverage, exemptions: &str) -> Vec<String> {
    let mut listed = std::collections::BTreeMap::new();
    let mut found = Vec::new();
    for line in exemptions.lines().filter(|line| !line.trim().is_empty() && !line.starts_with('#')) {
        match line.split_once('\t') {
            Some((kind, reason)) if !reason.trim().is_empty() => {
                listed.insert(kind.trim().to_owned(), reason.trim().to_owned());
            }
            _ => found.push(format!("unreachable kinds: `{line}` has no reason")),
        }
    }
    let uncovered = covered.uncovered();
    for kind in &uncovered {
        if !listed.contains_key(*kind) {
            found.push(format!("spelled kind `{kind}` is never seen with text in the fixtures and is not listed in {UNREACHABLE_KINDS}"));
        }
    }
    for kind in listed.keys() {
        if !covered.spelled.contains(kind) {
            found.push(format!("{UNREACHABLE_KINDS}: `{kind}` is not a spelled kind of the language"));
        } else if !uncovered.contains(&kind.as_str()) {
            found.push(format!("{UNREACHABLE_KINDS}: `{kind}` is covered by the fixtures: remove its entry"));
        }
    }
    found
}

#[cfg(test)]
mod coverage_tests {
    use super::{Coverage, coverage_failures};

    fn covered(spelled: &[&str], text: &[&str]) -> Coverage {
        let set = |names: &[&str]| names.iter().map(|name| (*name).to_owned()).collect();
        Coverage { spelled: set(spelled), occurring: set(text), text_old: set(text), text_walk: set(text), numbered: true }
    }

    #[test]
    fn a_spelled_kind_without_text_must_be_listed_with_a_reason() {
        let found = covered(&["a", "b"], &["a"]);
        assert_eq!(coverage_failures(&found, "").len(), 1, "b is uncovered and not listed");
        assert!(coverage_failures(&found, "b\tno source writes it").is_empty());
        assert_eq!(coverage_failures(&found, "b\t").len(), 2, "no reason: the entry is refused and b is still unlisted");
    }

    #[test]
    fn a_stale_entry_is_refused() {
        let found = covered(&["a", "b"], &["a", "b"]);
        assert_eq!(coverage_failures(&found, "b\tonce unreachable").len(), 1, "b is covered");
        assert_eq!(coverage_failures(&found, "c\tnot a kind").len(), 1, "c is not spelled");
    }
}

#[cfg(test)]
mod canonical_tests {
    use super::{Canon, Collect, Relation};

    /// The rows of an `ExpressionShape`: expression, representative, file, span (the last two as they come).
    fn rows(of: &[(u64, u64)]) -> Collect {
        let mut collected = Collect::default();
        for (expression, representative) in of {
            collected.relations.push(Relation::ExpressionShape);
            collected.words.extend([*expression, *representative, 0, 0]);
        }
        collected
    }

    /// Two routes number the same three segments (keys by file offsets) in different orders and pick their own member as the
    /// representative of the one class: after canonicalising, the rows are the same.
    #[test]
    fn two_numberings_of_the_same_embedded_segments_give_the_same_rows() {
        // Positions 10, 11, 12 are the segments at offsets 100, 200, 300 on one route; 12, 10, 11 on the other.
        let key_a = |at: u32| match at { 10 => (100, 110, 7), 11 => (200, 210, 7), _ => (300, 310, 7) };
        let key_b = |at: u32| match at { 12 => (100, 110, 7), 10 => (200, 210, 7), _ => (300, 310, 7) };
        let mut canon = Canon::default();
        let mut a = rows(&[(10, 11), (11, 11), (12, 11)]);
        canon.apply(&mut a, Relation::ExpressionShape, &key_a);
        let mut b = rows(&[(12, 10), (10, 10), (11, 10)]);
        canon.apply(&mut b, Relation::ExpressionShape, &key_b);
        let mut sorted = |collected: &Collect| {
            let mut rows: Vec<Vec<u64>> = collected.words.chunks(4).map(<[u64]>::to_vec).collect();
            rows.sort();
            rows
        };
        assert_eq!(sorted(&a), sorted(&b), "the representative is the least member whichever route met it first");
    }

    /// Other relations' node cells are still named by their own key, and two classes stay two classes.
    #[test]
    fn two_classes_stay_apart() {
        let key = |at: u32| (at * 10, at * 10 + 5, 7);
        let mut canon = Canon::default();
        let mut collected = rows(&[(1, 1), (2, 1), (3, 3)]);
        canon.apply(&mut collected, Relation::ExpressionShape, &key);
        let shapes: Vec<u64> = collected.words.chunks(4).map(|row| row[1]).collect();
        assert_eq!(shapes[0], shapes[1]);
        assert_ne!(shapes[0], shapes[2]);
    }
}

/// One line of `adjudicated_routes.tsv`: a difference between the routes that a person judged once, with the side that is right.
#[derive(Debug, Clone)]
struct Adjudged {
    path: String,
    relation: String,
    /// How many rows differ (only in old plus only in the fill): a new difference in the pair changes it and fails.
    rows: usize,
    /// Which route is right: `walk` or `old`.
    side: String,
    reason: String,
}

/// The adjudicated differences of a language: `tests/fixtures/adjudicated_routes.tsv`, one line each,
/// `<path>\t<relation>\t<rows>\t<class>\t<right side: walk or old>\t<reason>`. A difference on a listed (path, relation) whose
/// differing rows are exactly `rows` is not counted (it is reported as ADJUDICATED); one with another count is a difference; a line
/// whose path a run read and whose pair did not differ is stale and fails; a line without its six fields fails to load. The path is
/// the sample's path under the fixtures folder, or a corpus file's path under its root.
#[derive(Debug, Default)]
pub struct Adjudication {
    lines: Vec<Adjudged>,
    visited: Vec<bool>,
    used: Vec<bool>,
    /// How many differing pairs this run judged right on one side.
    pub adjudicated: usize,
}

/// The name of the list in a fixtures folder.
pub const ADJUDICATED_ROUTES: &str = "adjudicated_routes.tsv";

impl Adjudication {
    /// Reads the list; a missing file is an empty list.
    ///
    /// # Errors
    ///
    /// A line without its six fields, with a row count that is no number, a side that is neither `walk` nor `old`, or no reason.
    pub fn parse(text: &str) -> Result<Self, String> {
        let mut lines = Vec::new();
        for line in text.lines().filter(|line| !line.trim().is_empty() && !line.starts_with('#')) {
            let fields: Vec<&str> = line.split('\t').collect();
            let [path, relation, rows, class, side, reason] = fields.as_slice() else {
                return Err(format!("{ADJUDICATED_ROUTES}: `{line}` does not have its six fields (path, relation, rows, class, side, reason)"));
            };
            let rows = rows.trim().parse::<usize>().map_err(|_| format!("{ADJUDICATED_ROUTES}: `{line}` has no row count"))?;
            if class.trim().is_empty() || reason.trim().is_empty() || !matches!(side.trim(), "walk" | "old") {
                return Err(format!("{ADJUDICATED_ROUTES}: `{line}` needs a class, a side (walk or old) and a reason"));
            }
            lines.push(Adjudged { path: (*path).to_owned(), relation: (*relation).to_owned(), rows, side: (*side).trim().to_owned(), reason: (*reason).trim().to_owned() });
        }
        let count = lines.len();
        Ok(Self { lines, visited: vec![false; count], used: vec![false; count], adjudicated: 0 })
    }

    /// The list of a fixtures folder.
    ///
    /// # Errors
    ///
    /// As `parse`.
    pub fn load(fixtures: &Path) -> Result<Self, String> {
        Self::parse(&std::fs::read_to_string(fixtures.join(ADJUDICATED_ROUTES)).unwrap_or_default())
    }

    /// The side the list judged right for the pair (`walk` or `old`), when it lists it.
    pub(super) fn side_of(&self, path: &str, relation: &str) -> Option<&str> {
        self.lines.iter().find(|line| line.path == path && line.relation == relation).map(|line| line.side.as_str())
    }

    /// Notes that a run read `path`: its lines can be stale from now on.
    fn visit(&mut self, path: &str) {
        for (at, line) in self.lines.iter().enumerate() {
            if line.path == path {
                self.visited[at] = true;
            }
        }
    }

    /// What to report of a difference of `rows` rows in the pair: nothing when it is adjudicated as pinned, else `shown`.
    fn settle(&mut self, path: &str, relation: &str, rows: usize, shown: String) -> Option<String> {
        let Some(at) = self.lines.iter().position(|line| line.path == path && line.relation == relation) else {
            return Some(shown);
        };
        if self.lines[at].rows == rows {
            self.used[at] = true;
            self.adjudicated += 1;
            return None;
        }
        self.used[at] = true;
        Some(format!("{shown} [adjudicated pair, but {rows} rows differ and {} are pinned: {}]", self.lines[at].rows, self.lines[at].reason))
    }

    /// The lines of paths a run read whose pair did not differ: they no longer say anything true.
    #[must_use]
    pub fn stale(&self) -> Vec<String> {
        self.lines
            .iter()
            .enumerate()
            .filter(|(at, _)| self.visited[*at] && !self.used[*at])
            .map(|(_, line)| format!("{ADJUDICATED_ROUTES}: `{} {}` no longer differs: remove the line", line.path, line.relation))
            .collect()
    }
}

#[cfg(test)]
mod adjudication_tests {
    use super::Adjudication;

    const LINE: &str = "a.py\tBindingEvent\t2\t8\twalk\tthe walk follows source order";

    #[test]
    fn a_listed_pair_is_not_counted_and_a_changed_count_is() {
        let mut list = Adjudication::parse(LINE).expect("a well formed line");
        list.visit("a.py");
        assert_eq!(list.settle("a.py", "BindingEvent", 2, "shown".into()), None, "pinned: adjudicated");
        assert_eq!(list.adjudicated, 1);
        assert!(list.settle("a.py", "BindingEvent", 3, "shown".into()).is_some_and(|text| text.contains("3 rows differ")), "a new row fails");
        assert_eq!(list.settle("a.py", "Reference", 1, "other".into()), Some("other".into()), "an unlisted pair is a difference");
    }

    #[test]
    fn a_stale_line_and_a_line_without_a_reason_fail() {
        let mut list = Adjudication::parse(LINE).expect("a well formed line");
        assert!(list.stale().is_empty(), "a path the run never read is not stale");
        list.visit("a.py");
        assert_eq!(list.stale().len(), 1, "read, and no difference: stale");
        assert!(Adjudication::parse("a.py\tBindingEvent\t2\t8\twalk\t").is_err(), "no reason");
        assert!(Adjudication::parse("a.py\tBindingEvent\t2\t8\tneither\tr").is_err(), "no side");
    }
}
