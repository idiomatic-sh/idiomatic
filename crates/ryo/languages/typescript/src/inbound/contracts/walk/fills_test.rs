//! The fillers' cases: for each relation the language fills, over the sources of the language's fixtures, the rows its filler writes
//! for a walked file are the rows today's reader writes, cell by cell (an element by what it is and where it stands, a text by its
//! characters), and the first row that differs is shown.

use super::contract::WalkPorts;
use crate::domain::file_view::types::TypeScriptWalk;
use ryo::languages::{
    self, ByteSlice, CatalogRelation, CellKind, CellWord, FileFacts, FileId, FileVisit, FilledColumns, NodeTable, Offset,
    Position, Relation, RelationNeeds, Walk, WalkRequest, contract, facts, node_table,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// One element of a tree by what it is and where it stands: its kind and extent, and its place among the elements the same.
type ElementKey = (u16, u32, u32, u32);

/// One cell of a row, in a form two readings of a file can be compared in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Seen {
    Element(ElementKey),
    Other(u64),
    Text(String),
    Unknown(u8),
    NoText,
    File,
    Span(Offset, Offset),
    Number(i64),
    Flag(bool),
}

fn walk() -> TypeScriptWalk<WalkPorts> {
    TypeScriptWalk {
        ports: WalkPorts {
            syntax: Arc::new(crate::outbound::syntax::factory::oxc_parser()),
            embedded: vec![
                Arc::new(crate::outbound::embedded::factory::vue()),
                Arc::new(crate::outbound::embedded::factory::svelte()),
            ],
        },
    }
}

/// The sources of the fixtures: every typescript and javascript file under the corpus, the standard, the suite and the relations'.
fn sources() -> Vec<(PathBuf, Vec<u8>)> {
    fn collect(folder: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(folder) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect(&path, found);
            } else if matches!(path.extension().and_then(|e| e.to_str()), Some("ts" | "tsx" | "js" | "jsx" | "mjs" | "cts" | "mts")) {
                found.push(path);
            }
        }
    }
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut found = Vec::new();
    for folder in ["corpus", "standard", "suite", "allocation", "relations"] {
        collect(&root.join(folder), &mut found);
    }
    found.sort();
    found.into_iter().map(|path| { let bytes = std::fs::read(&path).expect("a fixture is read"); (path, bytes) }).collect()
}

/// The keys of elements given as (kind, start, end) in the order of the table: each with its place among the same.
fn keyed(elements: impl Iterator<Item = (u16, u32, u32)>) -> Vec<ElementKey> {
    let mut seen = std::collections::HashMap::<(u16, u32, u32), u32>::new();
    elements
        .map(|key| {
            let nth = seen.entry(key).or_insert(0);
            let found = (key.0, key.1, key.2, *nth);
            *nth += 1;
            found
        })
        .collect()
}

fn element(keys: &[ElementKey], word: u64) -> Seen {
    let position = usize::try_from(word & 0xffff_ffff).unwrap_or(usize::MAX);
    keys.get(position).map_or(Seen::Other(word & 0xffff_ffff), |key| Seen::Element(*key))
}

fn plain(kind: CellKind, word: u64) -> Seen {
    match kind {
        CellKind::Node => Seen::Other(word),
        CellKind::Symbol => Seen::NoText,
        CellKind::File => Seen::File,
        CellKind::Span => Seen::Span(
            Offset(u32::try_from(word >> 32).unwrap_or(u32::MAX)),
            Offset(u32::try_from(word & 0xffff_ffff).unwrap_or(u32::MAX)),
        ),
        CellKind::Number => Seen::Number(word.cast_signed()),
        CellKind::Flag => Seen::Flag(word != 0),
    }
}

/// A text cell that holds no text of the file: no text, or the unknown value of its reason.
fn special(word: u64) -> Option<Seen> {
    let symbol = languages::Symbol(u32::try_from(word).unwrap_or(u32::MAX));
    if symbol == languages::NO_SYMBOL {
        return Some(Seen::NoText);
    }
    let first_unknown = languages::unknown_symbol(languages::UnknownReason(0)).0;
    if symbol.0 >= first_unknown {
        return u8::try_from(symbol.0 - first_unknown).ok().map(Seen::Unknown);
    }
    use languages::{DeclarationKind, ParameterKind, Term as _};
    use strum::IntoEnumIterator as _;
    let first_term = DeclarationKind::iter().map(languages::term_symbol).chain(ParameterKind::iter().map(languages::term_symbol)).min()?;
    if symbol < first_term {
        return None;
    }
    DeclarationKind::iter()
        .find(|kind| languages::term_symbol(*kind) == symbol)
        .map(|kind| kind.text())
        .or_else(|| ParameterKind::iter().find(|kind| languages::term_symbol(*kind) == symbol).map(|kind| kind.text()))
        .map(|text| Seen::Text(text.0.to_owned()))
}

/// The rows today's reader writes for `relation`, sorted.
fn old_rows<R: CatalogRelation>(relation: Relation, path: &str, bytes: &[u8]) -> Vec<Vec<Seen>> {
    let linked = crate::linked();
    let mut elements = languages::Elements::default();
    let mut symbols = languages::Symbols::default();
    let mut out = contract::Collect::default();
    contract::read(
        linked.parse_engine().as_ref(),
        languages::ParseRequest {
            path: languages::SourcePath(path),
            file: FileId(0),
            bytes: ByteSlice(bytes),
            notes: languages::NoteRequest::Unwanted,
            fields: languages::FieldRequest::Wanted,
        },
        &mut elements,
        &mut symbols,
        &[relation],
        &mut out,
    )
    .expect("today's reading reads the source");
    let own = if languages::end_of(&elements).0 == 0 { 0 } else { usize::from(languages::last_at(&elements, Position(0))) + 1 };
    let keys = keyed((0..own).map(|at| {
        let at = Position(u32::try_from(at).unwrap_or(u32::MAX));
        (languages::kind_at(&elements, at).0, languages::start_at(&elements, at).0, languages::end_at(&elements, at).0)
    }));
    let kinds = R::kinds();
    let mut rows: Vec<Vec<Seen>> = out
        .rows()
        .into_iter()
        .filter(|(row_relation, _)| *row_relation == relation)
        .map(|(_, words)| {
            kinds
                .iter()
                .zip(&words)
                .map(|(kind, word)| match kind {
                    CellKind::Node => element(&keys, *word),
                    CellKind::Symbol => special(*word).unwrap_or_else(|| {
                        let symbol = languages::Symbol(u32::try_from(*word).unwrap_or(u32::MAX));
                        Seen::Text(languages::text_of(&symbols, ByteSlice(bytes), symbol).unwrap_or_default().0.to_owned())
                    }),
                    CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => plain(*kind, *word),
                })
                .collect()
        })
        .collect();
    rows.sort_unstable();
    rows
}

/// The visit that fills `relation` from the lent file.
struct Fill<'a, R> {
    relation: Relation,
    rows: &'a mut Vec<Vec<Seen>>,
    kind: std::marker::PhantomData<R>,
}

impl<R: CatalogRelation> FileVisit for Fill<'_, R> {
    fn visit(&mut self, file: FileFacts<'_>) {
        let table = file.table;
        let keys = keyed((0..node_table::element_count(table).0).map(|index| {
            let at = Position(u32::try_from(index).unwrap_or(u32::MAX));
            let span = node_table::span_of(table, at);
            (node_table::kind_of(table, at).0, span.start.0, span.end.0)
        }));
        let mut words = facts::file_words(&facts::base_words(languages::fixed_names::checked(&[]).expect("no fixed name"), &[]));
        let mut columns = FilledColumns::default();
        let _answer = file.facts.fill(self.relation, languages::Wanted::Whole, &mut words, &mut columns);
        let kinds = R::kinds();
        let count = columns.0.first().map_or(0, |column| column.0.len());
        *self.rows = (0..count)
            .map(|row| {
                kinds
                    .iter()
                    .zip(&columns.0)
                    .map(|(kind, column)| {
                        let word = column.0.get(row).copied().unwrap_or(CellWord(0)).0;
                        match kind {
                            CellKind::Node => element(&keys, word),
                            CellKind::Symbol => special(word).unwrap_or_else(|| Seen::Text(facts::text(&words, CellWord(word)).0.to_owned())),
                            CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => plain(*kind, word),
                        }
                    })
                    .collect()
            })
            .collect();
        self.rows.sort_unstable();
    }
}

fn new_rows<R: CatalogRelation>(relation: Relation, path: &str, bytes: &[u8]) -> Vec<Vec<Seen>> {
    let walk = walk();
    let mut table = NodeTable::default();
    let mut rows = Vec::new();
    let mut visit = Fill::<R> { relation, rows: &mut rows, kind: std::marker::PhantomData };
    walk.walk(
        WalkRequest { path: languages::SourcePath(path), file: FileId(0), source: ByteSlice(bytes) },
        RelationNeeds(&[]),
        &mut node_table::builder(&mut table, <TypeScriptWalk<WalkPorts> as Walk>::KIND_COUNT),
        &mut (),
        &mut visit,
    )
    .expect("the walk reads the source");
    rows
}

/// What differs between the rows today's reader writes and the filler's, over every fixture source: the first differing rows of
/// each source, and how many rows the sources hold in all.
fn differences<R: CatalogRelation>(relation: Relation) -> (String, usize) {
    let mut report = String::new();
    let mut total = 0;
    for (path, bytes) in sources() {
        let name = format!("input.{}", path.extension().and_then(|e| e.to_str()).unwrap_or("ts"));
        let (old, new) = (old_rows::<R>(relation, &name, &bytes), new_rows::<R>(relation, &name, &bytes));
        total += old.len();
        if old != new {
            let lacking = |side: &str, have: &[Vec<Seen>], other: &[Vec<Seen>]| {
                have.iter().filter(|each| !other.contains(each)).take(2).map(|each| format!("  {side} only: {each:?}\n")).collect::<String>()
            };
            report.push_str(&format!(
                "{}: reader {} rows, filler {}\n{}{}",
                path.display(),
                old.len(),
                new.len(),
                lacking("reader", &old, &new),
                lacking("filler", &new, &old),
            ));
        }
    }
    (report, total)
}

/// Holds one relation's filler to its reader.
fn equal<R: CatalogRelation>(relation: Relation) {
    let (found, total) = differences::<R>(relation);
    assert!(found.is_empty(), "{relation:?}: {found}");
    println!("{relation:?}: {total} rows equal");
}

#[test]
fn comment_fills_equal_reads() {
    equal::<languages::Comment>(Relation::Comment);
}

#[test]
fn numeric_literal_fills_equal_reads() {
    equal::<languages::NumericLiteral>(Relation::NumericLiteral);
}

#[test]
fn string_literal_fills_equal_reads() {
    equal::<languages::StringLiteral>(Relation::StringLiteral);
}

#[test]
fn parameter_fills_equal_reads() {
    equal::<languages::Parameter>(Relation::Parameter);
}

#[test]
fn call_argument_fills_equal_reads() {
    equal::<languages::CallArgument>(Relation::CallArgument);
}
