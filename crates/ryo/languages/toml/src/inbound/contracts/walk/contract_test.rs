//! The `walk` contract's oracle: over every TOML source of the language's fixtures and of the Ryo workspace's own manifests, the walk
//! writes the tree today's reading writes (every element, under the same parent, at the same place, in the same field), and the
//! fillers write the rows today's readers write, relation by relation. Today's table numbers nodes and tokens together in
//! pre-order and the walk's numbers the nodes first, so an element is compared by what it is and where it is: its kind, its span,
//! and its place among the elements that share both.

use crate::domain::file_view::types::{TomlScratch, TomlWalk};
use crate::inbound::contracts::walk::contract::WalkPorts;
use crate::outbound::syntax::factory::toml_parser;
use ryo::languages::{
    self, ByteSlice, CatalogRelation, CellKind, CellWord, FactAnswer, FileFacts, FileId, FileVisit,
    FilledColumns, KindBits, MappingKind, NodeTable, Offset, Position, Relation, RelationNeeds, Term as _,
    ValueKind, Walk, WalkRequest, contract, facts, node_table,
};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use strum::IntoEnumIterator as _;

/// An element by what it is and where: its kind, its span, and its place among the elements of the file that share both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key {
    kind: KindBits,
    start: Offset,
    end: Offset,
    nth: u32,
}

/// One cell of a row, in a form two readings of a file can be compared in.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
enum Seen {
    Element(Key),
    Other(u64),
    Text(String),
    NoText,
    File,
    Span(Offset, Offset),
    Number(i64),
    Flag(bool),
}

/// One element of a tree where it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Placement {
    element: Key,
    parent: Key,
    ordinal: u32,
    field: u16,
}

/// The relations the language fills, and compares.
const FILLED: [Relation; 7] = [
    Relation::Comment,
    Relation::Entry,
    Relation::Mapping,
    Relation::SequenceElement,
    Relation::SourceLocation,
    Relation::StringLiteral,
    Relation::SyntaxError,
];

/// The kinds of the columns of each relation compared.
fn column_kinds(relation: Relation) -> &'static [CellKind] {
    match relation {
        Relation::Comment => <languages::Comment as CatalogRelation>::kinds(),
        Relation::Entry => <languages::Entry as CatalogRelation>::kinds(),
        Relation::Mapping => <languages::Mapping as CatalogRelation>::kinds(),
        Relation::SequenceElement => <languages::SequenceElement as CatalogRelation>::kinds(),
        Relation::SourceLocation => <languages::SourceLocation as CatalogRelation>::kinds(),
        Relation::StringLiteral => <languages::StringLiteral as CatalogRelation>::kinds(),
        Relation::SyntaxError => <languages::SyntaxError as CatalogRelation>::kinds(),
        _ => &[],
    }
}

fn keyed(elements: impl Iterator<Item = (KindBits, Offset, Offset)>) -> Vec<Key> {
    let mut seen: HashMap<(KindBits, Offset, Offset), u32> = HashMap::new();
    elements
        .map(|(kind, start, end)| {
            let nth = seen.entry((kind, start, end)).or_insert(0);
            let key = Key { kind, start, end, nth: *nth };
            *nth += 1;
            key
        })
        .collect()
}

fn old_keys(elements: &languages::Elements) -> Vec<Key> {
    keyed((0..languages::end_of(elements).0).map(|at| {
        (
            languages::kind_at(elements, Position(at)),
            languages::start_at(elements, Position(at)),
            languages::end_at(elements, Position(at)),
        )
    }))
}

fn new_keys(table: &NodeTable) -> Vec<Key> {
    let count = u32::try_from(node_table::element_count(table).0).unwrap_or(u32::MAX);
    keyed((0..count).map(|id| {
        let span = node_table::span_of(table, Position(id));
        (node_table::kind_of(table, Position(id)), span.start, span.end)
    }))
}

fn old_placements(elements: &languages::Elements, keys: &[Key]) -> Vec<Placement> {
    let mut placements: Vec<Placement> = keys
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let at = Position(u32::try_from(index).unwrap_or(u32::MAX));
            let field = languages::field_at(elements, at);
            Placement {
                element: *element,
                parent: keys.get(usize::from(languages::parent_at(elements, at))).copied().unwrap_or(*element),
                ordinal: languages::ordinal_at(elements, at).0,
                field: if field == languages::NO_FIELD { 0 } else { field.0 },
            }
        })
        .collect();
    placements.sort_unstable();
    placements
}

fn new_placements(table: &NodeTable, keys: &[Key]) -> Vec<Placement> {
    let mut placements: Vec<Placement> = keys
        .iter()
        .enumerate()
        .map(|(index, element)| {
            let id = Position(u32::try_from(index).unwrap_or(u32::MAX));
            Placement {
                element: *element,
                parent: keys.get(usize::from(node_table::parent_of(table, id))).copied().unwrap_or(*element),
                ordinal: node_table::ordinal_of(table, id).0,
                field: node_table::field_of(table, id).0,
            }
        })
        .collect();
    placements.sort_unstable();
    placements
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

fn element(keys: &[Key], word: u64) -> Seen {
    let position = usize::try_from(word & 0xffff_ffff).unwrap_or(usize::MAX);
    keys.get(position).map_or(Seen::Other(word & 0xffff_ffff), |key| Seen::Element(*key))
}

/// A word that is no text of the file: no text, or a word of a closed vocabulary (a value's kind, a mapping's).
fn special(word: u64) -> Option<Seen> {
    let symbol = languages::Symbol(u32::try_from(word).unwrap_or(u32::MAX));
    if symbol == languages::NO_SYMBOL {
        return Some(Seen::NoText);
    }
    ValueKind::iter()
        .find(|kind| languages::term_symbol(*kind) == symbol)
        .map(|kind| kind.text())
        .or_else(|| MappingKind::iter().find(|kind| languages::term_symbol(*kind) == symbol).map(|kind| kind.text()))
        .map(|text| Seen::Text(format!("term {}", text.0)))
}

fn old_rows(
    rows: &HashMap<Relation, Vec<Vec<u64>>>,
    read: &Old,
    bytes: &[u8],
    keys: &[Key],
    relation: Relation,
) -> Vec<Vec<Seen>> {
    let kinds = column_kinds(relation);
    let mut seen: Vec<Vec<Seen>> = rows
        .get(&relation)
        .map(Vec::as_slice)
        .unwrap_or_default()
        .iter()
        .map(|words| {
            kinds
                .iter()
                .zip(words)
                .map(|(kind, word)| match kind {
                    CellKind::Node => element(keys, *word),
                    CellKind::Symbol => special(*word).unwrap_or_else(|| {
                        let symbol = languages::Symbol(u32::try_from(*word).unwrap_or(u32::MAX));
                        Seen::Text(
                            languages::text_of(&read.symbols, ByteSlice(bytes), symbol).unwrap_or_default().0.to_owned(),
                        )
                    }),
                    CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => plain(*kind, *word),
                })
                .collect()
        })
        .collect();
    seen.sort_unstable();
    seen
}

fn new_rows(
    file: &mut FileFacts<'_>,
    keys: &[Key],
    relation: Relation,
    words: &mut languages::FileWords,
) -> (Vec<Vec<Seen>>, FactAnswer) {
    let mut columns = FilledColumns::default();
    let answer = file.facts.fill(relation, languages::Wanted::Whole, words, &mut columns);
    let kinds = column_kinds(relation);
    let count = columns.0.first().map_or(0, |column| column.0.len());
    let mut rows: Vec<Vec<Seen>> = (0..count)
        .map(|row| {
            kinds
                .iter()
                .zip(&columns.0)
                .map(|(kind, column)| {
                    let word = column.0.get(row).copied().unwrap_or(CellWord(0)).0;
                    match kind {
                        CellKind::Node => element(keys, word),
                        CellKind::Symbol => special(word)
                            .unwrap_or_else(|| Seen::Text(facts::text(words, CellWord(word)).0.to_owned())),
                        CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => plain(*kind, word),
                    }
                })
                .collect()
        })
        .collect();
    rows.sort_unstable();
    (rows, answer)
}

/// Today's reading of one file: its table, its names and every row it wrote.
#[derive(Default)]
struct Old {
    elements: languages::Elements,
    symbols: languages::Symbols,
    rows: contract::Collect,
}

/// A visit of a lent file by a function.
struct Visit<'v>(&'v mut dyn FnMut(FileFacts<'_>));

impl FileVisit for Visit<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        (self.0)(file);
    }
}

/// The sources the oracle runs over: the language's fixtures and every TOML manifest of the Ryo workspace.
fn sources() -> Vec<PathBuf> {
    fn gather(dir: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            if path.is_dir() {
                if !matches!(name.as_str(), "target" | ".git" | "node_modules") {
                    gather(&path, found);
                }
            } else if name.ends_with(".toml") || name == "Cargo.lock" {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    gather(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."), &mut found);
    found.sort();
    found
}

/// What the walk and today's reading differ in over every source, one line for each finding, and the table of rows by relation.
fn compared() -> (String, String) {
    let language = crate::linked();
    let engine = language.parse_engine();
    let walk = TomlWalk { ports: WalkPorts { syntax: Arc::new(toml_parser()) } };
    let mut findings = String::new();
    let mut totals: HashMap<String, (usize, usize, usize, usize)> = HashMap::new();
    let mut files = 0_usize;
    let mut table = NodeTable::default();
    let mut scratch = TomlScratch::default();
    let mut old = Old::default();
    let names = <TomlWalk<WalkPorts> as languages::Fillers>::vocabulary().fixed;
    for path in sources() {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let shown = path.display().to_string();
        let named = shown.rsplit("../../").next().unwrap_or(&shown).to_owned();
        let source_path = path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        old.rows.clear();
        let read = contract::read(
            &*engine,
            languages::ParseRequest {
                path: languages::SourcePath(&source_path),
                file: FileId(0),
                bytes: ByteSlice(&bytes),
                notes: languages::NoteRequest::Unwanted,
                fields: languages::FieldRequest::Wanted,
            },
            &mut old.elements,
            &mut old.symbols,
            &FILLED,
            &mut old.rows,
        );
        if read.is_err() {
            continue;
        }
        files += 1;
        let by_relation = {
            let mut by: HashMap<Relation, Vec<Vec<u64>>> = HashMap::new();
            for (relation, words) in old.rows.rows() {
                by.entry(relation).or_default().push(words);
            }
            by
        };
        let old_key_list = old_keys(&old.elements);
        let old_syntax = languages::has_incomplete_syntax(&old.elements);
        let old_lexical = languages::has_incomplete_lexical_stream(&old.elements);
        let old_places = old_placements(&old.elements, &old_key_list);
        let mut builder = node_table::builder(&mut table, TomlWalk::<WalkPorts>::KIND_COUNT);
        let walked = walk.walk(
            WalkRequest { path: languages::SourcePath(&source_path), file: FileId(0), source: ByteSlice(&bytes) },
            RelationNeeds(&FILLED),
            &mut builder,
            &mut scratch,
            &mut Visit(&mut |mut lent: FileFacts<'_>| {
                let walked = lent.facts.completeness();
                if (walked.syntax, walked.lexical) != (old_syntax, old_lexical) {
                    let _ = writeln!(
                        findings,
                        "{named}: completeness differs: old syntax {old_syntax:?} lexical {old_lexical:?}, walk syntax {:?} lexical {:?}",
                        walked.syntax, walked.lexical
                    );
                }
                let keys = new_keys(lent.table);
                let places = new_placements(lent.table, &keys);
                if places != old_places {
                    let first = places.iter().zip(&old_places).find(|(a, b)| a != b);
                    let _ = writeln!(
                        findings,
                        "{named}: tree differs: {} placements walked, {} read; first {first:?}",
                        places.len(),
                        old_places.len()
                    );
                }
                let mut words = facts::file_words(&facts::base_words(
                    languages::fixed_names::checked(&names.0).unwrap_or_default(),
                    &[],
                ));
                for relation in FILLED {
                    let before = old_rows(&by_relation, &old, &bytes, &old_key_list, relation);
                    let (after, answer) = new_rows(&mut lent, &keys, relation, &mut words);
                    if matches!(answer, FactAnswer::Partial(_))
                        && node_table::acceptance_of(lent.table) == languages::SyntaxAcceptance::Accepted
                    {
                        let _ = writeln!(findings, "{named}: {relation:?} answers in part in a file the parser accepted");
                    }
                    let total = totals.entry(format!("{relation:?}")).or_default();
                    total.0 += before.len();
                    total.1 += after.len();
                    if before == after {
                        total.2 += 1;
                    } else {
                        total.3 += 1;
                        let only_old = before.iter().find(|row| !after.contains(row));
                        let only_new = after.iter().find(|row| !before.contains(row));
                        let _ = writeln!(
                            findings,
                            "{named}: {} differs ({answer:?}): only today: {only_old:?}; only walked: {only_new:?}",
                            format!("{relation:?}")
                        );
                    }
                }
            }),
        );
        if let Err(error) = walked {
            let _ = writeln!(findings, "{named}: walk refused: {error:?}");
        }
    }
    let mut report = format!("{files} files\nrelation | rows old | rows filled | files equal | files differing\n");
    let mut names: Vec<_> = totals.into_iter().collect();
    names.sort();
    for (name, (old_rows, new_rows, equal, differing)) in names {
        let _ = writeln!(report, "{name} | {old_rows} | {new_rows} | {equal} | {differing}");
    }
    (findings, report)
}

#[test]
fn the_walk_and_its_fillers_write_what_todays_reading_writes() {
    let (findings, report) = compared();
    println!("{report}");
    assert!(findings.is_empty(), "{report}\n{}", findings.lines().take(20).collect::<Vec<_>>().join("\n"));
}
