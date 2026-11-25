//! The `walk` contract's cases: over the sources of the language's fixtures, the walk writes the tree today's reading writes (every
//! element, under the same parent, at the same place, in the same field), and its kinds and fields are named at the codes the table holds.

use super::contract::WalkPorts;
use crate::domain::file_view::types::TypeScriptWalk;
use crate::domain::fields::service::{field_bits, field_name, fields};
use crate::domain::syntax::service::{kind_bits, kind_name, kind_of, kinds};
use crate::outbound::embedded::factory;
use crate::outbound::notes::factory::postcard;
use crate::outbound::syntax::factory::oxc_parser;
use ryo::languages::{
    self, ByteSlice, Fillers, FileFacts, FileId, FileVisit, NodeTable, Position, RelationNeeds, Walk, WalkRequest, contract,
    node_table,
};
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// The walk over the real ports.
fn walk() -> TypeScriptWalk<WalkPorts> {
    let _notes = postcard();
    TypeScriptWalk {
        ports: WalkPorts {
            syntax: Arc::new(oxc_parser()),
            embedded: vec![Arc::new(factory::vue()), Arc::new(factory::svelte())],
        },
    }
}

/// An element by what it is and where it stands: its kind and its extent.
type Element = (u16, u32, u32);

/// Where an element stands: itself, its parent, its place among its parent's children, and its field.
type Placement = (Element, Element, u32, u16);

/// Today's tree of one source: every element of the file's own tree and where it stands.
fn today(path: &str, bytes: &[u8]) -> Vec<Placement> {
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
        &[],
        &mut out,
    )
    .expect("today's reading reads the source");
    let own = if languages::end_of(&elements).0 == 0 { 0 } else { usize::from(languages::last_at(&elements, Position(0))) + 1 };
    let keys: Vec<Element> = (0..own)
        .map(|at| {
            let at = Position(u32::try_from(at).unwrap_or(u32::MAX));
            (languages::kind_at(&elements, at).0, languages::start_at(&elements, at).0, languages::end_at(&elements, at).0)
        })
        .collect();
    let mut placements: Vec<Placement> = (0..own)
        .map(|index| {
            let at = Position(u32::try_from(index).unwrap_or(u32::MAX));
            let parent = languages::parent_at(&elements, at);
            let field = languages::field_at(&elements, at);
            (
                keys[index],
                keys.get(usize::from(parent)).copied().unwrap_or(keys[index]),
                languages::ordinal_at(&elements, at).0,
                if field == languages::NO_FIELD { 0 } else { field.0 },
            )
        })
        .collect();
    placements.sort_unstable();
    placements
}

/// The tree the walk writes for one source: every element and where it stands.
fn walked(path: &str, bytes: &[u8]) -> Vec<Placement> {
    let walk = walk();
    let mut table = NodeTable::default();
    let mut placements = Vec::new();
    let mut visit = Capture(&mut placements);
    walk.walk(
        WalkRequest { path: languages::SourcePath(path), file: FileId(0), source: ByteSlice(bytes) },
        RelationNeeds(&[]),
        &mut node_table::builder(&mut table, <TypeScriptWalk<WalkPorts> as Walk>::KIND_COUNT),
        &mut (),
        &mut visit,
    )
    .expect("the walk reads the source");
    placements.sort_unstable();
    placements
}

/// The visit that reads the placements of the lent table.
struct Capture<'a>(&'a mut Vec<Placement>);

impl FileVisit for Capture<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        let table = file.table;
        let count = node_table::element_count(table).0;
        let keys: Vec<Element> = (0..count)
            .map(|index| {
                let at = Position(u32::try_from(index).unwrap_or(u32::MAX));
                let span = node_table::span_of(table, at);
                (node_table::kind_of(table, at).0, span.start.0, span.end.0)
            })
            .collect();
        for (index, key) in keys.iter().enumerate() {
            let at = Position(u32::try_from(index).unwrap_or(u32::MAX));
            let parent = node_table::parent_of(table, at);
            self.0.push((
                *key,
                keys.get(usize::from(parent)).copied().unwrap_or(*key),
                node_table::ordinal_of(table, at).0,
                node_table::field_of(table, at).0,
            ));
        }
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

/// What differs between the walk's tree and today's, over every fixture source: the first differing elements of each.
fn differences() -> String {
    let mut report = String::new();
    let sources = sources();
    assert!(sources.len() >= 30, "the fixtures hold their sources: {} found", sources.len());
    let mut compared = 0;
    for (path, bytes) in sources {
        let name = format!("input.{}", path.extension().and_then(|e| e.to_str()).unwrap_or("ts"));
        let (old, new) = (today(&name, &bytes), walked(&name, &bytes));
        compared += old.len();
        if old != new {
            let lacking = |side: &str, have: &[Placement], other: &[Placement]| {
                have.iter().filter(|each| !other.contains(each)).take(3).map(|each| format!("{side} only: {each:?}\n")).collect::<String>()
            };
            report.push_str(&format!(
                "{}: today {} elements, walk {}\n{}{}",
                path.display(),
                old.len(),
                new.len(),
                lacking("today", &old, &new),
                lacking("walk", &new, &old),
            ));
        }
    }
    assert!(compared > 5_000, "the sources hold elements to compare: {compared}");
    report
}

#[test]
fn the_walk_writes_the_tree_todays_reading_writes() {
    let found = differences();
    assert!(found.is_empty(), "{found}");
}

#[test]
fn the_kind_count_is_the_count_of_the_languages_kinds() {
    assert_eq!(<TypeScriptWalk<WalkPorts> as Walk>::KIND_COUNT.0, kinds().count());
}

#[test]
fn the_walk_names_each_kind_and_each_field_at_its_code() {
    let names = <TypeScriptWalk<WalkPorts> as Fillers>::vocabulary();
    for kind in kinds() {
        assert_eq!(kind_of(kind_bits(kind)), kind);
        assert_eq!(names.kinds.0.get(usize::from(kind_bits(kind).0)), Some(&kind_name(kind)));
    }
    for field in fields() {
        assert_eq!(names.fields.0.get(usize::from(field_bits(field).0)), Some(&field_name(field)));
    }
}
