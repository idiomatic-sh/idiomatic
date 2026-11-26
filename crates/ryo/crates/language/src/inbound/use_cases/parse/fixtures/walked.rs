//! A file read through the walk, as the conformance holds it: every relation the language answers written as rows, the node-table
//! relations as the numbered adapter lends them, and the kinds of the elements of the tree. The rows name a node by its place in the
//! file's table, as the fixtures' expected rows do.

use super::super::collect::Collect;
use super::super::types::{FieldRequest, NoteRequest, ParseEngine, ParseRequest};
use super::rows::Read;
use crate::domain::facts::service::{base_words, file_words};
use crate::domain::facts::types::{FactSource, FileFacts, FileWords};
use crate::domain::files::types::SourcePath;
use crate::domain::fixed_names::service::checked;
use crate::domain::node_table::service::{children_of, field_of, is_token, kind_codes, ordinal_of, own_end, parent_of, span_of};
use crate::domain::symbols::service::intern_derived;
use crate::domain::symbols::types::{ByteSlice, Symbols};
use crate::inbound::use_cases::walk::types::{FileVisit, RelationNeeds, WalkEngine, WalkRequest};
use ryo_relations::domain::relations::service::all;
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{Bytes, CellKind, FileId, Position, Relation};
use ryo_relations::domain::terms::types::NameText;

/// What the walk and the adapter gave for one file.
pub(super) struct WalkedFile {
    /// The rows, by relation, and what they need to be shown.
    pub read: Read,
    /// The kind of each position of the table and whether it is a LEAF: a token, or an element with no children (the walk contract's
    /// leaf rule: a provider may write a leaf either way, and Rust writes it as a childless element).
    pub elements: Vec<(u16, bool)>,
    /// What the language leaves in the notes of the file.
    pub notes: Bytes,
}

/// The four relations the node table holds: no filler writes them, the numbered adapter lends them.
const NODE_TABLE: [Relation; 4] = [Relation::SyntaxTree, Relation::SyntaxChild, Relation::SemanticSyntax, Relation::SemanticChild];

struct Collecting<'a> {
    words: FileWords,
    path: SourcePath<'a>,
    source: ByteSlice<'a>,
    out: Collect,
    symbols: Symbols,
    elements: Vec<(u16, bool)>,
    notes: Bytes,
}

impl FileVisit for Collecting<'_> {
    fn visit(&mut self, file: FileFacts<'_>) {
        let FileFacts { facts, table, .. } = file;
        let count = kind_codes(table).len();
        self.elements = (0..count)
            .map(|at| {
                let place = Position(u32::try_from(at).unwrap_or(u32::MAX));
                (kind_codes(table)[at], is_token(table, place) || children_of(table, place).is_empty())
            })
            .collect();
        facts.notes(table, self.source, &mut self.notes);
        for &relation in all() {
            if NODE_TABLE.contains(&relation) {
                continue;
            }
            let facts: &mut dyn FactSource = &mut *facts;
            let mut columns = crate::domain::facts::types::FilledColumns::default();
            match facts.fill(relation, crate::domain::facts::types::Wanted::Whole, &mut self.words, &mut columns) {
                crate::domain::facts::types::FactAnswer::Refused(_) => continue,
                crate::domain::facts::types::FactAnswer::Partial(partial) => {
                    self.out.partials.push((relation, partial.cause.0.to_string(), partial.missing));
                }
                crate::domain::facts::types::FactAnswer::Answered(_) => {}
            }
            let rows = columns.0.first().map_or(0, |column| column.0.len());
            for row in 0..rows {
                self.out.relations.push(relation);
                for (kind, column) in kinds(relation).iter().zip(&columns.0) {
                    let bits = column.0[row].0;
                    self.out.words.push(if *kind == CellKind::Symbol { super::routes::symbol_bits(&self.words, &mut self.symbols, self.source, bits) } else { bits });
                }
            }
        }
        let _ = self.path;
    }
}

/// Reads `bytes` as the file at `path` through the walk, and the node-table relations through the numbered adapter of `parse`.
///
/// # Errors
///
/// When the walk refuses the file.
pub(super) fn walk_file(parse: &dyn ParseEngine, walk: &dyn WalkEngine, path: &str, bytes: Vec<u8>) -> Result<WalkedFile, String> {
    let vocabulary = walk.vocabulary();
    let base = base_words(checked(&vocabulary.fixed.0).map_err(|error| format!("the walk's vocabulary: {error:?}"))?, &[]);
    let (mut table, mut buffers) = (crate::domain::node_table::types::NodeTable::default(), walk.buffers());
    let request = WalkRequest { path: SourcePath(path), file: FileId(0), source: ByteSlice(&bytes) };
    let mut collecting = Collecting {
        words: file_words(&base),
        path: SourcePath(path),
        source: ByteSlice(&bytes),
        out: Collect::default(),
        symbols: Symbols::default(),
        elements: Vec::new(),
        notes: Bytes::default(),
    };
    walk.walk_file(request, RelationNeeds(all()), &mut table, &mut *buffers, &mut collecting).map_err(|error| format!("{error:?}"))?;
    let Collecting { mut out, mut symbols, elements, notes, .. } = collecting;
    adapter_rows(parse, walk, SourcePath(path), ByteSlice(&bytes), &mut out, &mut symbols);
    Ok(WalkedFile {
        read: Read { out, symbols, texts: vec![bytes.clone()], bytes, files: Vec::new(), unknowns: walk.unknown_reasons() },
        elements,
        notes,
    })
}

/// The four node-table relations of the file as the numbered adapter lends them, named by the place of each node.
fn adapter_rows(parse: &dyn ParseEngine, walk: &dyn WalkEngine, path: SourcePath<'_>, source: ByteSlice<'_>, out: &mut Collect, symbols: &mut Symbols) {
    let numbered = parse.file_relations().numbered();
    let kind_names = walk.vocabulary().kinds;
    let request = ParseRequest { path, file: FileId(0), bytes: source, notes: NoteRequest::Unwanted, fields: FieldRequest::Wanted };
    let mut reader = numbered.open();
    let _ = reader.visit(request, all(), &[], None, symbols, &mut |file, symbols| {
        let table = file.tree;
        let count = kind_codes(table).len();
        let own = usize::try_from(own_end(table).0).unwrap_or(count).min(count);
        for at in 0..own {
            let position = Position(u32::try_from(at).unwrap_or(u32::MAX));
            let name = kind_names.0.get(usize::from(kind_codes(table)[at])).copied().unwrap_or(NameText(""));
            let span = span_of(table, position);
            out.relations.push(Relation::SyntaxTree);
            out.words.extend([
                u64::from(position.0),
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
                    u64::from(parent.0),
                    u64::from(position.0),
                    u64::from(ordinal_of(table, position).0),
                    u64::from(intern_derived(symbols, source, field).0),
                    0,
                ]);
            }
        }
        let mut semantic = Collect::default();
        let _ = file.facts.project_structure(&[Relation::SemanticSyntax, Relation::SemanticChild], symbols, &mut semantic);
        let mut rest = semantic.words.as_slice();
        for each in &semantic.relations {
            let (row, after) = rest.split_at(kinds(*each).len());
            rest = after;
            out.relations.push(*each);
            out.words.extend_from_slice(row);
        }
    });
}
