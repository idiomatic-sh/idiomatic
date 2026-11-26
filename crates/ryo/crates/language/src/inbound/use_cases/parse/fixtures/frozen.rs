//! The frozen record of a language's two routes: what the routes row holds a language to, written down as digests, so that the
//! comparison of the walk with the frozen rows can outlive the old readers.
//!
//! A record is four texts: the rows of every sample and relation (`rows.digests`), the completeness of every sample
//! (`completeness.tsv`), the node-table relations and the word of every position as the numbered adapter gives them
//! (`adapter.digests`), and what the language states about itself (`metadata.tsv`). A digest is of the SORTED rows as the routes row
//! renders them, with a node named by a hash of its place, so that it does not depend on the order the rows or the nodes are met in.
//! A record is made from the old reader (`Side::Old`) only when the walk agrees with it everywhere (`record`); the test holds the
//! walk to the record by making the same texts from the walk (`Side::Walk`).

use super::super::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest, Recognition};
use super::collect::Collect;
use super::routes::{Canon, Walked, canon_nodes, old_completeness, old_reading, walked};
use super::samples::samples_in;
use crate::domain::facts::service::{base_words, file_words};
use crate::domain::facts::types::{FactSource, FileFacts, FileWords};
use crate::domain::files::types::SourcePath;
use crate::domain::fixed_names::service::checked;
use crate::domain::node_table::service::{field_of, kind_codes, ordinal_of, own_end, parent_of, span_of};
use crate::domain::node_table::types::NodeTable;
use crate::domain::symbols::service::{intern_derived, text_of};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::walk::types::{FileVisit, RelationNeeds, WalkEngine, WalkRequest};
use ryo_relations::domain::relations::service::{all, kinds};
use ryo_relations::domain::relations::types::{CellKind, FileId, Position, Relation};
use ryo_relations::domain::terms::types::NameText;
use sha2::{Digest as _, Sha256};
use std::fmt::Write as _;
use std::path::Path;

/// Which route a record is made from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    /// The old reader: what a record is made from.
    Old,
    /// The walk: what the test makes to hold against the record.
    Walk,
}

/// The four texts of a record.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Frozen {
    /// `rows.digests`: a line per sample and relation the language answers: path, relation, row count, digest.
    pub rows: String,
    /// `completeness.tsv`: a line per sample: path, syntax, lexical.
    pub completeness: String,
    /// `adapter.digests`: a line per sample for each of the four node-table relations and for the words of the positions.
    pub adapter: String,
    /// `metadata.tsv`: what the language states of itself, and how it recognises each sample.
    pub metadata: String,
}

/// The place of a node, as a number that names it: a hash of its kind and its extent.
pub(super) fn key_hash(key: (u32, u32, u16)) -> u32 {
    let mut hasher = Sha256::new();
    hasher.update(key.0.to_le_bytes());
    hasher.update(key.1.to_le_bytes());
    hasher.update(key.2.to_le_bytes());
    let bytes = hasher.finalize();
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

/// The digest of a set of rendered rows: sorted, so that the order they were written in does not matter.
fn digest(mut rows: Vec<String>, causes: &[&str]) -> (usize, String) {
    rows.sort_unstable();
    let count = rows.len();
    let mut hasher = Sha256::new();
    for row in &rows {
        hasher.update(row.as_bytes());
        hasher.update(b"\n");
    }
    for cause in causes {
        hasher.update(b"partial: ");
        hasher.update(cause.as_bytes());
        hasher.update(b"\n");
    }
    let bytes = hasher.finalize();
    let mut hex = String::new();
    for byte in &bytes[..16] {
        let _ = write!(hex, "{byte:02x}");
    }
    (count, hex)
}

fn rendered(rows: &[&[super::wanted::Want]]) -> Vec<String> {
    rows.iter().map(|row| format!("{row:?}")).collect()
}

/// The records of every sample of `fixtures`, from `side`.
///
/// # Panics
///
/// When a folder of the fixtures cannot be read.
#[must_use]
pub fn frozen(old: &dyn ParseEngine, walk: &dyn WalkEngine, fixtures: &Path, side: Side) -> Frozen {
    let samples = {
        let mut samples = samples_in(fixtures);
        samples.sort_by_key(|sample| sample.path.0);
        samples
    };
    let mut record = Frozen::default();
    for sample in &samples {
        let (path, source) = (sample.path, sample.bytes);
        let lines = match side {
            Side::Old => {
                // The relations the language answers are the ones the walk's fillers write.
                let relations: Vec<Relation> = walk_lines(old, walk, path, source, false).rows.iter().map(|line| line.0).collect();
                old_lines(old, path, source, &relations, true)
            }
            Side::Walk => walk_lines(old, walk, path, source, true),
        };
        for (relation, count, hex) in lines.rows {
            let _ = writeln!(record.rows, "{}\t{relation:?}\t{count}\t{hex}", path.0);
        }
        let _ = writeln!(record.completeness, "{}\t{:?}\t{:?}", path.0, lines.completeness.0, lines.completeness.1);
        for (what, count, hex) in lines.adapter {
            let _ = writeln!(record.adapter, "{}\t{what}\t{count}\t{hex}", path.0);
        }
    }
    record.metadata = metadata(old, walk, side, &samples);
    record
}

/// What the language states of itself, from `side`, and how it recognises each sample.
fn metadata(old: &dyn ParseEngine, walk: &dyn WalkEngine, side: Side, samples: &[super::super::types::SourceFile]) -> String {
    let mut text = String::new();
    for &relation in all() {
        let (absence, expands) = match side {
            Side::Old => (old.absence_of(relation), old.includes_expansions(relation)),
            Side::Walk => (walk.absence_of(relation), walk.includes_expansions(relation)),
        };
        let _ = writeln!(text, "relation\t{relation:?}\t{absence:?}\t{expands}");
    }
    match side {
        Side::Old => {
            let _ = writeln!(text, "skips\t{:?}", old.skips());
            let _ = writeln!(text, "configuration\t{:?}", old.configuration_identity());
            let _ = writeln!(text, "capabilities\t{:?}", old.capabilities());
            let _ = writeln!(text, "unknown-reasons\t{:?}", old.unknown_reasons());
        }
        Side::Walk => {
            let _ = writeln!(text, "skips\t{:?}", walk.skips());
            let _ = writeln!(text, "configuration\t{:?}", walk.configuration_identity());
            let _ = writeln!(text, "capabilities\t{:?}", walk.capabilities());
            let _ = writeln!(text, "unknown-reasons\t{:?}", walk.unknown_reasons());
        }
    }
    for sample in samples {
        let recognised = match side {
            Side::Old => {
                let request = ParseRequest { path: sample.path, file: FileId(0), bytes: sample.bytes, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
                old.recognition(&request) == Recognition::Reads
            }
            Side::Walk => walk.recognizes(sample.path, false) == Recognition::Reads,
        };
        let _ = writeln!(text, "recognizes\t{}\t{recognised}", sample.path.0);
    }
    text
}

/// What one sample gives, from one route.
struct Lines {
    rows: Vec<(Relation, usize, String)>,
    completeness: (String, String),
    adapter: Vec<(String, usize, String)>,
}

const NODE_TABLE: [Relation; 4] = [Relation::SyntaxTree, Relation::SyntaxChild, Relation::SemanticSyntax, Relation::SemanticChild];

/// The lines of one sample from the old reader.
fn old_lines(old: &dyn ParseEngine, path: SourcePath<'_>, source: ByteSlice<'_>, relations: &[Relation], with_adapter: bool) -> Lines {
    let mut canon = Canon::hashed();
    let mut rows = Vec::new();
    for &relation in relations {
        if let Ok(reading) = old_reading(old, &mut canon, path, source, relation) {
            let (count, hex) = digest(rendered(reading.rows), reading.causes);
            rows.push((relation, count, hex));
        }
    }
    let completeness = old_completeness(old, path, source).map_or_else(
        || ("refused".to_owned(), "refused".to_owned()),
        |(syntax, lexical)| (format!("{syntax:?}"), format!("{lexical:?}")),
    );
    let mut adapter = Vec::new();
    for relation in NODE_TABLE.into_iter().filter(|_| with_adapter) {
        if let Ok(reading) = old_reading(old, &mut canon, path, source, relation) {
            let (count, hex) = digest(rendered(reading.rows), reading.causes);
            adapter.push((format!("{relation:?}"), count, hex));
        }
    }
    if with_adapter {
        adapter.push(("words".to_owned(), 0, old_words(old, path, source)));
    }
    Lines { rows, completeness, adapter }
}

/// The word of every position as the old reading holds it: the text of each element, by the element's place.
fn old_words(old: &dyn ParseEngine, path: SourcePath<'_>, source: ByteSlice<'_>) -> String {
    let mut symbols = Symbols::default();
    let mut elements = crate::domain::elements::types::Elements::default();
    let request = ParseRequest { path, file: FileId(0), bytes: source, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
    if super::super::service::read(old, request, &mut elements, &mut symbols, &[Relation::SyntaxTree], &mut Collect::default()).is_err() {
        return "refused".to_owned();
    }
    let mut spelled = Vec::new();
    for at in 0..crate::domain::elements::service::end_of(&elements).0 {
        let place = Position(at);
        let key = (
            crate::domain::elements::service::start_at(&elements, place).0,
            crate::domain::elements::service::end_at(&elements, place).0,
            crate::domain::elements::service::kind_at(&elements, place).0,
        );
        let word = text_of(&symbols, source, crate::domain::elements::service::text_at(&elements, place)).map(|text| text.0.to_owned());
        spelled.push(format!("{key:?} {word:?}"));
    }
    let (count, hex) = digest(spelled, &[]);
    format!("{count}:{hex}")
}

/// The lines of one sample from the walk.
fn walk_lines(old: &dyn ParseEngine, walk: &dyn WalkEngine, path: SourcePath<'_>, source: ByteSlice<'_>, with_adapter: bool) -> Lines {
    let vocabulary = walk.vocabulary();
    let base = base_words(checked(&vocabulary.fixed.0).expect("walk vocabulary"), &[]);
    let (mut table, mut buffers) = (NodeTable::default(), walk.buffers());
    let request = WalkRequest { path, file: FileId(0), source };
    let mut collect = Walking { words: file_words(&base), path, source, unknowns: walk.unknown_reasons(), rows: Vec::new(), completeness: None };
    let _ = walk.walk_file(request, RelationNeeds(all()), &mut table, &mut *buffers, &mut collect);
    let completeness = collect.completeness.map_or_else(
        || ("refused".to_owned(), "refused".to_owned()),
        |(syntax, lexical)| (format!("{syntax:?}"), format!("{lexical:?}")),
    );
    let mut adapter = Vec::new();
    if with_adapter {
        let (adapter_rows, words) = adapter_walk(old, walk, path, source);
        adapter = adapter_rows;
        adapter.push(("words".to_owned(), 0, words));
    }
    Lines { rows: collect.rows, completeness, adapter }
}

struct Walking<'a> {
    words: FileWords,
    path: SourcePath<'a>,
    source: ByteSlice<'a>,
    unknowns: &'static [ryo_relations::domain::terms::types::UnknownName],
    rows: Vec<(Relation, usize, String)>,
    completeness: Option<(crate::domain::diagnostics::types::SyntaxCompleteness, crate::domain::diagnostics::types::SyntaxCompleteness)>,
}

impl FileVisit for Walking<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        let FileFacts { facts, table, .. } = file;
        let complete = facts.completeness();
        self.completeness = Some((complete.syntax, complete.lexical));
        let mut canon = Canon::hashed();
        for &relation in all() {
            let facts: &mut dyn FactSource = &mut *facts;
            if let Walked::Rows(reading) = walked(relation, table, facts, &mut self.words, (self.path, self.source), self.unknowns, &mut canon) {
                let (count, hex) = digest(rendered(reading.rows), reading.causes);
                self.rows.push((relation, count, hex));
            }
        }
    }
}

/// The four node-table relations and the words of the positions as the numbered adapter lends them.
fn adapter_walk(old: &dyn ParseEngine, walk: &dyn WalkEngine, path: SourcePath<'_>, source: ByteSlice<'_>) -> (Vec<(String, usize, String)>, String) {
    let Some(numbered) = old.file_relations().and_then(|relations| relations.numbered()) else {
        return (Vec::new(), "none".to_owned());
    };
    let kind_names = walk.vocabulary().kinds;
    let request = ParseRequest { path, file: FileId(0), bytes: source, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
    let mut canon = Canon::hashed();
    let mut out = Collect::default();
    let mut symbols = Symbols::default();
    let mut spelled = Vec::new();
    let mut reader = numbered.open();
    let visited = reader.visit(request, all(), &[], None, &mut symbols, &mut |file, symbols| {
        let table = file.tree;
        let count = kind_codes(table).len();
        let own = usize::try_from(own_end(table).0).unwrap_or(count).min(count);
        let key = |at: u32| -> (u32, u32, u16) {
            if at as usize >= count {
                return (u32::MAX, u32::MAX, u16::MAX);
            }
            let span = span_of(table, Position(at));
            (span.start.0, span.end.0, kind_codes(table)[at as usize])
        };
        for at in 0..count {
            let word = file.words.get(at).filter(|word| **word != ryo_relations::domain::relations::constants::NO_SYMBOL).and_then(|word| text_of(symbols, source, *word)).map(|text| text.0.to_owned());
            spelled.push(format!("{:?} {word:?}", key(u32::try_from(at).unwrap_or(u32::MAX))));
        }
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
        return (Vec::new(), "refused".to_owned());
    }
    let read = super::rows::Read {
        out,
        symbols,
        bytes: source.0.to_vec(),
        texts: vec![source.0.to_vec()],
        files: Vec::new(),
        unknowns: old.unknown_reasons(),
    };
    let mut lines = Vec::new();
    for relation in NODE_TABLE {
        let rows = super::wanted::kept(super::rows::rows_of_relation(&read, relation, path.0));
        let (count, hex) = digest(rendered(rows), &[]);
        lines.push((format!("{relation:?}"), count, hex));
    }
    let (count, hex) = digest(spelled, &[]);
    (lines, format!("{count}:{hex}"))
}

/// The record of the language's fixtures, made from the old reader, or why it cannot be made: the routes differ somewhere, and a
/// difference is never frozen.
///
/// # Errors
///
/// The differences between the two routes, when there is any.
pub fn record(old: &dyn ParseEngine, walk: &dyn WalkEngine, fixtures: &Path) -> Result<Frozen, Vec<String>> {
    let differences = super::routes::fixture_route_differences(old, walk, fixtures);
    if !differences.is_empty() {
        return Err(differences);
    }
    let from_old = frozen(old, walk, fixtures, Side::Old);
    let from_walk = frozen(old, walk, fixtures, Side::Walk);
    // The record is made from the old reader; the walk must say the same in every line, which the route differences already say
    // of the rows: the adapter's words and the completeness are held here.
    let mut differing = Vec::new();
    for (what, old_text, walk_text) in [
        ("rows", &from_old.rows, &from_walk.rows),
        ("completeness", &from_old.completeness, &from_walk.completeness),
        ("adapter", &from_old.adapter, &from_walk.adapter),
        ("metadata", &from_old.metadata, &from_walk.metadata),
    ] {
        for (old_line, walk_line) in old_text.lines().zip(walk_text.lines()) {
            if old_line != walk_line {
                differing.push(format!("{what}: old {old_line:?}, walk {walk_line:?}"));
            }
        }
        if old_text.lines().count() != walk_text.lines().count() {
            differing.push(format!("{what}: {} lines from the old reader, {} from the walk", old_text.lines().count(), walk_text.lines().count()));
        }
    }
    if differing.is_empty() { Ok(from_old) } else { Err(differing) }
}

/// The tag the failure of the frozen comparison names: the commit just before the old readers were deleted, where the rows of both
/// routes can be made again.
pub const TAG: &str = "pre-old-readers-deletion";

/// The files of the language under `roots`, at any depth, not entering its skipped directories or `.git`, as the corpus
/// comparison finds them: the path shown and the bytes, in path order.
///
/// # Panics
///
/// When a root cannot be read.
#[must_use]
pub fn corpus_files(old: &dyn ParseEngine, roots: &[std::path::PathBuf]) -> Vec<(String, Vec<u8>)> {
    let skipped: Vec<&str> = old.skips().iter().map(|each| each.0).collect();
    let mut found = Vec::new();
    let mut stack: Vec<(std::path::PathBuf, std::path::PathBuf)> = roots.iter().map(|root| (root.clone(), root.clone())).collect();
    while let Some((root, path)) = stack.pop() {
        let Ok(entries) = std::fs::read_dir(&path) else {
            assert!(path != root, "a corpus root cannot be read: {}", path.display());
            continue;
        };
        for entry in entries.filter_map(Result::ok).map(|entry| entry.path()) {
            let name = entry.file_name().and_then(|name| name.to_str()).unwrap_or_default().to_owned();
            if entry.is_dir() {
                if name != ".git" && !skipped.contains(&name.as_str()) {
                    stack.push((root.clone(), entry));
                }
                continue;
            }
            let Ok(bytes) = std::fs::read(&entry) else { continue };
            let shown = entry.strip_prefix(&root).unwrap_or(&entry).to_string_lossy().replace('\\', "/");
            let request = ParseRequest { path: SourcePath(&shown), file: FileId(0), bytes: ByteSlice(&bytes), notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
            if old.recognition(&request) == Recognition::Reads {
                found.push((shown, bytes));
            }
        }
    }
    found.sort();
    found
}

/// The lines of a corpus, from `side`: a line for a file and relation only where the relation has rows (no line means no rows).
#[must_use]
pub fn corpus_text(old: &dyn ParseEngine, walk: &dyn WalkEngine, files: &[(String, Vec<u8>)], side: Side) -> String {
    let mut text = String::new();
    for (shown, bytes) in files {
        let (path, source) = (SourcePath(shown), ByteSlice(bytes));
        let lines = match side {
            Side::Old => {
                let relations: Vec<Relation> = walk_lines(old, walk, path, source, false).rows.iter().map(|line| line.0).collect();
                old_lines(old, path, source, &relations, false)
            }
            Side::Walk => walk_lines(old, walk, path, source, false),
        };
        for (relation, count, hex) in lines.rows.into_iter().filter(|line| line.1 > 0) {
            let _ = writeln!(text, "{shown}\t{relation:?}\t{count}\t{hex}");
        }
    }
    text
}

/// Writes one frozen file: header lines, then the body. The header names what the file is, the reason it was recorded and the
/// number of lines of the body, so that a truncation is seen.
///
/// # Panics
///
/// When the file cannot be written.
pub fn write_file(path: &Path, what: &str, language: &str, reason: &str, body: &str) {
    if let Some(folder) = path.parent() {
        std::fs::create_dir_all(folder).expect("the folder of the frozen files is made");
    }
    let text = format!(
        "# {what} of {language}, recorded from the old reader (routes-rerecord.sh); reason: {reason}\n# lines: {}\n{body}",
        body.lines().count()
    );
    std::fs::write(path, text).expect("a frozen file is written");
}

/// The differences between `body` and the frozen file `path`: the first differing line with both digests and counts, or what is
/// wrong with the file. Empty when they agree. A failure says how to get the rows: the tag `TAG`.
#[must_use]
pub fn differences_from_file(path: &Path, body: &str) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else {
        return vec![format!("{}: the frozen file is missing: record it with routes-rerecord.sh", path.display())];
    };
    let (mut header, mut frozen) = (Vec::new(), Vec::new());
    for line in text.lines() {
        if line.starts_with('#') { header.push(line) } else { frozen.push(line) }
    }
    let mut found = Vec::new();
    let expected = header.iter().find_map(|line| line.strip_prefix("# lines: ")).and_then(|count| count.parse::<usize>().ok());
    if expected != Some(frozen.len()) {
        found.push(format!("{}: the header says {expected:?} lines, the file holds {}: it was cut", path.display(), frozen.len()));
    }
    let computed: Vec<&str> = body.lines().collect();
    let first = frozen.iter().zip(&computed).position(|(frozen, computed)| frozen != computed);
    if let Some(at) = first {
        found.push(format!(
            "{}: line {} differs: frozen {:?}, walk {:?} ({} lines frozen, {} from the walk); the rows of both routes can be made again at the tag `{TAG}`",
            path.display(), at + 1, frozen[at], computed[at], frozen.len(), computed.len()
        ));
    } else if frozen.len() != computed.len() {
        found.push(format!("{}: {} lines frozen, {} from the walk; the rows of both routes can be made again at the tag `{TAG}`", path.display(), frozen.len(), computed.len()));
    }
    found
}
