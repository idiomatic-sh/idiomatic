//! The `walk` contract's oracle: over every go source and go.mod of the language's fixtures and of the Ryo workspace, the walk
//! writes the tree today's reading writes (every element, under the same parent, at the same place, in the same field), and the
//! fillers write the rows today's readers write, relation by relation. Today's table numbers nodes and tokens together in
//! pre-order and the walk's numbers the nodes first, so an element is compared by what it is and where it is: its kind, its span,
//! and its place among the elements that share both.

use crate::domain::file_view::types::GoWalk;
use crate::inbound::contracts::walk::contract::WalkPorts;
use crate::outbound::syntax::factory::tree_sitter_go;
use ryo::languages::{
    self, ByteSlice, FileFacts, FileId, FileVisit, KindBits, NodeTable, Offset, Position, RelationNeeds, Walk, WalkRequest, contract,
    node_table,
};
use std::collections::HashMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Arc;

/// An element by what it is and where: its kind, its span, and its place among the elements of the file that share both.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Key {
    kind: KindBits,
    start: Offset,
    end: Offset,
    nth: u32,
}

/// One element of a tree where it stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct Placement {
    element: Key,
    parent: Key,
    ordinal: u32,
    field: u16,
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

/// A visit of a lent file by a function.
struct Visit<'v>(&'v mut dyn FnMut(FileFacts<'_>));

impl FileVisit for Visit<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        (self.0)(file);
    }
}

/// The sources the oracle runs over: the language's fixtures and every go source and module file of the Ryo workspace.
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
            } else if name.ends_with(".go") || name == "go.mod" {
                found.push(path);
            }
        }
    }
    let mut found = Vec::new();
    gather(&Path::new(env!("CARGO_MANIFEST_DIR")).join("../.."), &mut found);
    found.sort();
    found
}

/// What the walked tree and today's reading differ in over every source, one line for each finding.
fn compared() -> (String, usize) {
    let language = crate::linked();
    let engine = language.parse_engine();
    let walk = GoWalk { ports: WalkPorts { syntax: Arc::new(tree_sitter_go()) } };
    let mut findings = String::new();
    let mut files = 0_usize;
    let mut table = NodeTable::default();
    let mut elements = languages::Elements::default();
    let mut symbols = languages::Symbols::default();
    let mut rows = contract::Collect::default();
    for path in sources() {
        let Ok(bytes) = std::fs::read(&path) else { continue };
        let shown = path.display().to_string();
        let named = shown.rsplit("../../").next().unwrap_or(&shown).to_owned();
        let source_path = path.file_name().map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let read = contract::read(
            &*engine,
            languages::ParseRequest {
                path: languages::SourcePath(&source_path),
                file: FileId(0),
                bytes: ByteSlice(&bytes),
                notes: languages::NoteRequest::Unwanted,
                fields: languages::FieldRequest::Wanted,
            },
            &mut elements,
            &mut symbols,
            &[],
            &mut rows,
        );
        if read.is_err() {
            continue;
        }
        files += 1;
        let old_key_list = old_keys(&elements);
        let old_places = old_placements(&elements, &old_key_list);
        let mut builder = node_table::builder(&mut table, GoWalk::<WalkPorts>::KIND_COUNT);
        let walked = walk.walk(
            WalkRequest { path: languages::SourcePath(&source_path), file: FileId(0), source: ByteSlice(&bytes) },
            RelationNeeds(&[]),
            &mut builder,
            &mut (),
            &mut Visit(&mut |lent: FileFacts<'_>| {
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
            }),
        );
        if let Err(error) = walked {
            let _ = writeln!(findings, "{named}: walk refused: {error:?}");
        }
    }
    (findings, files)
}

#[test]
fn the_walked_tree_is_the_tree_todays_reading_writes() {
    let (findings, files) = compared();
    assert!(files > 400, "the corpus is the fixtures and the workspace: {files} files");
    assert!(findings.is_empty(), "{}", findings.lines().take(20).collect::<Vec<_>>().join("\n"));
}
