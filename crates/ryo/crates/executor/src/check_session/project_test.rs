//! A relation that needs a file's notes, read in the visit of the file from its own notes, gives the file the rows the project phase
//! gives it from every file's notes together. A fake language answers both ways and names nothing of any real one.

use super::{Authority, Contribution, collect, layout};
use crate::check_session::{FileSource, Result, World, finish, l};
use ryo_language::domain::files::types::ProjectEntries;
use ryo_language::domain::symbols::service::intern_derived;
use ryo_relations::domain::relations::collections::relations::project_namespace_membership::item::ProjectNamespaceMembershipRow;
use ryo_relations::domain::relations::types::{FileId, Rows};
use std::rc::Rc;

/// The relation both reads answer: one File column, the notes of the file needed, no node or span.
const MEMBERSHIP: l::Relation = l::Relation::ProjectNamespaceMembership;

/// A language whose notes of a file are the text of one root name, and which answers `MEMBERSHIP` from them.
#[derive(Clone)]
struct Notes {
    /// Whether it answers a file alone (otherwise it answers `Unsupported`, as the default does).
    alone: bool,
}

impl Notes {
    /// The row of the file `file` whose notes are `notes`: the root it names, present as a module.
    fn write(&self, symbols: &mut l::Symbols, rows: &mut l::contract::Collect, file: usize, notes: &l::Bytes) {
        if notes.0.is_empty() {
            return;
        }
        let root = String::from_utf8_lossy(&notes.0).into_owned();
        let mut text = |word: &str| intern_derived(symbols, l::ByteSlice(b""), l::NameText(word));
        let row = ProjectNamespaceMembershipRow {
            file: FileId(u32::try_from(file).expect("a file number")),
            root: text(&root),
            path: text(&format!("{root}.part")),
            membership: text("present"),
            kind: text("module"),
        };
        rows.write(row);
    }
}

impl FileSource for Notes {
    fn name(&self) -> &str { "fake" }
    fn identity(&self) -> (&str, &str) { ("fake", "") }
    fn reads(&self, path: &str) -> bool { path.ends_with(".x") }
    fn manifests(&self) -> Vec<&'static str> { Vec::new() }
    fn words(&self) -> Vec<l::UnknownName> { Vec::new() }
    fn kinds(&self) -> Vec<&'static str> { Vec::new() }
    fn expands(&self, _: l::Relation) -> bool { false }
    fn available(&self, _: l::Relation) -> bool { true }
    fn absent(&self, _: l::Relation) -> bool { false }
    fn entries(&self, _: &l::ProjectRequest, _: &mut ProjectEntries) -> Result<()> { Ok(()) }
    fn project(&self, req: &l::ProjectRequest, symbols: &mut l::Symbols, rows: &mut l::contract::Collect) -> Result<l::Resolved> {
        for (file, notes) in req.notes.iter().enumerate() {
            self.write(symbols, rows, file, notes);
        }
        Ok(resolved())
    }
    fn file_rows(
        &self,
        _: &l::ProjectRequest,
        file: usize,
        notes: &l::Bytes,
        _: &l::ObservedEntries,
        symbols: &mut l::Symbols,
        rows: &mut l::contract::Collect,
    ) -> Result<Option<l::Resolved>> {
        if !self.alone {
            return Ok(None);
        }
        self.write(symbols, rows, file, notes);
        Ok(Some(resolved()))
    }
}

fn resolved() -> l::Resolved {
    l::Resolved { refused: Vec::new(), unlisted: Vec::new(), consulted: Vec::new(), observed_entries: Vec::new(), derived: () }
}

const FILES: [&str; 4] = ["a.x", "b.x", "c.x", "d.x"];

/// The notes of each file: a name for three of them, nothing for `c.x`.
fn notes_of(path: &str) -> l::Bytes {
    match path {
        "a.x" => l::Bytes(b"alpha".as_slice().into()),
        "b.x" => l::Bytes(b"beta".as_slice().into()),
        "d.x" => l::Bytes(b"delta".as_slice().into()),
        _ => l::Bytes::default(),
    }
}

/// The rows of `MEMBERSHIP` the project phase gives each file, as text: the root, the path, the membership and the kind it reads.
fn from_the_phase(language: &Notes) -> Vec<Vec<String>> {
    let mut world = World::default();
    world.declare_words(std::slice::from_ref(language));
    let inventory: Vec<String> = FILES.iter().map(|path| (*path).to_owned()).collect();
    let contributions: Vec<Rc<Contribution>> = FILES
        .iter()
        .map(|path| Rc::new(Contribution { reader: Some(0), notes: notes_of(path), manifest: None }))
        .collect();
    let population = collect(&mut world, std::slice::from_ref(language), &Authority::Given, &inventory, None, &contributions, &[MEMBERSHIP])
        .expect("the project phase answers");
    let table = population.tables.get(ryo_relations::domain::relations::service::name(MEMBERSHIP).0).expect("the relation");
    text_of_rows(&world.run, table)
}

fn text_of_rows(run: &finish::Run, table: &finish::Table) -> Vec<Vec<String>> {
    (0..table.n)
        .map(|row| {
            table
                .cols
                .iter()
                .zip(table.tys.iter())
                .map(|(column, ty)| match ty {
                    finish::Ty::Sym => String::from_utf8_lossy(&run.texts[column[row] as usize]).into_owned(),
                    _ => column[row].to_string(),
                })
                .collect()
        })
        .collect()
}

#[test]
fn a_relation_read_in_the_visit_gives_each_file_the_rows_the_project_phase_gives_it() {
    let language = Notes { alone: true };
    let inventory: Vec<String> = FILES.iter().map(|path| (*path).to_owned()).collect();
    let layout = layout(std::slice::from_ref(&language), &Authority::Given, &inventory, None, &[MEMBERSHIP])
        .expect("a language that answers one file alone gets the relation read in the visit");
    assert!(layout.reads_in_visit());
    let mut run = World::default().run;
    let mut world = World::default();
    world.declare_words(std::slice::from_ref(&language));
    let public = crate::reader::public_words(&mut run, &[]);
    let aliases = layout.declare(&mut run, "test");
    let mut visited = Vec::new();
    for (at, path) in FILES.iter().enumerate() {
        let read = layout
            .read_file(Some(&language), &public, &mut run, &aliases, path, l::FileId(at as u32), &notes_of(path))
            .expect("the file is read");
        let tables = layout.slice(&mut run, &aliases, path, l::FileId(at as u32), Some(&read));
        for (_, table) in tables {
            visited.extend(text_of_rows(&run, &table).into_iter().map(|mut row| {
                // The file cell is the file's number in each reading: it is not compared.
                row.remove(0);
                row
            }));
        }
    }
    let phase: Vec<Vec<String>> = from_the_phase(&language).into_iter().map(|mut row| { row.remove(0); row }).collect();
    assert_eq!(visited, phase, "the rows of the files, read one file at a time, are the rows of the project phase");
    assert_eq!(visited.len(), 3, "a file with no notes has no row");
}

#[test]
fn a_language_that_does_not_answer_one_file_alone_keeps_its_relation_on_the_closed_path() {
    let language = Notes { alone: false };
    let inventory: Vec<String> = FILES.iter().map(|path| (*path).to_owned()).collect();
    assert!(
        layout(std::slice::from_ref(&language), &Authority::Given, &inventory, None, &[MEMBERSHIP]).is_none(),
        "no layout: the program goes through the closed project phase"
    );
}
