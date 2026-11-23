//! THROWAWAY PROTOTYPE (ryo-fmx3.1, `claude/engine-cols`): the provider's facts written DIRECTLY as per-node columns and keyed
//! lists, beside the reader (never checked in; the reader's rows stay the reference). The engine asks, for one walked file, a
//! list of facts, each by its key column and either whole or as a yes/no question; the provider answers each with one
//! `FactTable`: a per-node table (one value per node, indexed by node id, with a presence bit a node) or a keyed list (all rows
//! of a key contiguous, an offset a node). Texts are written as the engine's own text ids through `TextIds`, memoised per model
//! name and per fixed text, so a row costs no per-file interning and no conversion after. Nothing here names a linter or a rule.

use crate::domain::definitions::constants as refinements;
use crate::domain::extras::types::Extra as TypedExtra;
use crate::domain::file_view::types::{Analysed, PythonFile, PythonWalk};
use crate::domain::language::service::reads;
use crate::domain::lexical_facts::service::qualify;
use crate::domain::lexical_facts::types::{NameProvenance, QualifiedPath};
use crate::domain::literal_facts::service::{number, properties, specifier as loaded, string_value};
use crate::domain::literal_facts::types::StringValue;
use crate::domain::model::flow::types::FlowState;
use crate::domain::model::lexical::service::entry;
use crate::domain::model::lexical::types::{NameUse, NodeClass, TypedTree};
use crate::domain::model::names::service::text_of;
use crate::domain::model::names::types::NameCode;
use crate::domain::model::view::service::built;
use crate::domain::model::view::types::{Built, ModelPart, ModelView, PartState};
use crate::domain::node::service::{nodes_of_kind, table_id};
use crate::domain::node::types::{Numbered, SyntaxFile, Walked, WalkedTree};
use crate::domain::nodes::types::{Category, Node};
use crate::domain::qualification::constants::{AMBIGUOUS, BUILTIN, LOCAL, NOT_APPLICABLE, UNRESOLVED};
use crate::domain::relations::collections::relations::expression_type::constants as type_labels;
use crate::domain::expression_types::types::Resolution;
use crate::domain::shape_facts::service::shapes;
use crate::domain::shape_facts::types::ShapeBuild;
use crate::domain::string_annotations::service::{form_imports, type_strings};
use crate::domain::string_annotations::types::FormSource;
use crate::domain::syntax::service::{category_of, kind_of};
use crate::domain::syntax::types::Kind;
use crate::domain::type_facts::service::resolutions;
use crate::inbound::contracts::parse::service::source_kind;
use crate::inbound::contracts::walk::contract::{PythonWalkScratch, WalkPorts};
use crate::outbound::syntax::factory::ruff_python_parser;
use crate::outbound::syntax::interface::SyntaxPort as _;
use crate::outbound::syntax::types::{ReportedSyntax, SourceText};
use crate::settings::PythonParserSettings;
use ruff_python_ast::AnyNodeRef;
use ryo::languages::{
    self, CatalogRelation, Column, FactAnswer, FactSource as _, FileId, FileWords, FilledColumns, Flag,
    NameText, NodeTable, Position, Relation, Wanted, WalkError, fillers, node_table,
};
use std::collections::HashMap;
use std::sync::Arc;

/// The engine's text ids: the provider writes every text cell as the id the engine gives the text.
pub trait TextIds {
    /// The id of `text`.
    fn id(&mut self, text: &[u8]) -> u32;
}

/// How a fact is laid out.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Layout {
    /// One row at most a node: a column's value at a node is the row's, and `present` says which nodes have one.
    #[default]
    PerNode,
    /// Any number of rows a node: the rows of node `k` are `offsets[k]..offsets[k + 1]`.
    Keyed,
}

/// One fact asked of a file: a relation, the column its rows are found by, and whether only "has this node a row" is wanted.
#[derive(Clone, Copy, Debug)]
pub struct Ask {
    /// The relation.
    pub relation: Relation,
    /// The place, among the relation's catalog columns, of the node column its rows are found by.
    pub key: usize,
    /// Only the yes/no question: the presence bits, and no column.
    pub exists: bool,
    /// The catalog places of the columns wanted, one bit a place (`u64::MAX`: all): a column no test reads is not written.
    pub columns: u64,
}

/// One fact of one file, as the provider wrote it.
#[derive(Clone, Debug, Default)]
pub struct FactTable {
    /// The layout.
    pub layout: Layout,
    /// Whether the provider answered the fact whole for this file (false: the file was not read whole, a superset holds).
    pub answered: bool,
    /// Per node, one bit: the node has a row (both layouts, so the yes/no question is one bit read).
    pub present: Vec<u64>,
    /// `Keyed`: per node that has rows (in node order), where its rows start in `order`; one more entry at the end.
    pub offsets: Vec<u32>,
    /// `Keyed`: per word of `present`, how many nodes before it have rows.
    pub rank: Vec<u32>,
    /// `Keyed`: the rows by key, as row numbers into the columns (empty: the rows came in key order, row `i` is `order[i]`).
    pub order: Vec<u32>,
    /// By catalog place, the values: by node (`PerNode`) or by row (`Keyed`); empty for a column not written.
    pub columns: Vec<Vec<u32>>,
    /// How many rows.
    pub rows: usize,
    /// `Keyed`, while writing: each row's key node.
    keys: Vec<u32>,
    /// The columns wanted, one bit a catalog place.
    wanted: u64,
}

impl FactTable {
    fn begin(&mut self, layout: Layout, width: usize, elements: usize, written: &[usize], exists: bool, wanted: u64) {
        self.wanted = wanted;
        self.layout = layout;
        self.answered = true;
        self.rows = 0;
        self.present.clear();
        self.present.resize(elements.div_ceil(64).max(1), 0);
        self.offsets.clear();
        self.order.clear();
        self.keys.clear();
        self.columns.resize_with(width.max(self.columns.len()), Vec::new);
        for column in &mut self.columns {
            column.clear();
        }
        if layout == Layout::PerNode && !exists {
            for place in written {
                if wanted >> place & 1 == 0 {
                    continue;
                }
                if let Some(column) = self.columns.get_mut(*place) {
                    column.resize(elements, 0);
                }
            }
        }
    }

    #[inline]
    fn wants(&self, place: usize) -> bool {
        self.wanted >> place & 1 == 1
    }

    #[inline]
    fn mark(&mut self, node: u32) {
        if let Some(word) = self.present.get_mut(node as usize / 64) {
            *word |= 1 << (node % 64);
        }
    }

    /// A per-node row: its values by catalog place.
    #[inline]
    fn put(&mut self, node: u32, values: &[(usize, u32)]) {
        self.mark(node);
        self.keys.push(node);
        self.rows += 1;
        for (place, value) in values {
            if let Some(cell) = self.columns.get_mut(*place).and_then(|column| column.get_mut(node as usize)) {
                *cell = *value;
            }
        }
    }

    /// A keyed row.
    #[inline]
    fn push(&mut self, node: u32, values: &[(usize, u32)]) {
        self.mark(node);
        self.keys.push(node);
        self.rows += 1;
        for (place, value) in values {
            if self.wanted >> place & 1 == 1
                && let Some(column) = self.columns.get_mut(*place)
            {
                column.push(*value);
            }
        }
    }

    /// Ends a keyed table: the rows are put in key order by a sort of (key, row) pairs, O(rows log rows), and indexed by
    /// key through the presence bits and a rank a word, O(elements / 64): no pass over the file's elements.
    fn finish(&mut self, _elements: usize) {
        if self.layout != Layout::Keyed {
            return;
        }
        self.rank.clear();
        let mut before = 0u32;
        for word in &self.present {
            self.rank.push(before);
            before += word.count_ones();
        }
        self.offsets.clear();
        self.order.clear();
        if self.keys.is_sorted() {
            let mut last = u32::MAX;
            for (row, key) in self.keys.iter().enumerate() {
                if *key != last {
                    self.offsets.push(row as u32);
                    last = *key;
                }
            }
        } else {
            let mut pairs: Vec<u64> = self.keys.iter().enumerate().map(|(row, key)| u64::from(*key) << 32 | row as u64).collect();
            pairs.sort_unstable();
            let mut last = u32::MAX;
            for (at, pair) in pairs.iter().enumerate() {
                let key = (pair >> 32) as u32;
                if key != last {
                    self.offsets.push(at as u32);
                    last = key;
                }
                self.order.push(*pair as u32);
            }
        }
        self.offsets.push(self.keys.len() as u32);
    }

    /// The rows of `node`, as places into `order` (or into the columns when `order` is empty); none when it has no row.
    #[inline]
    #[must_use]
    pub fn rows_of(&self, node: usize) -> std::ops::Range<usize> {
        let Some(word) = self.present.get(node / 64) else { return 0..0 };
        let bit = 1u64 << (node % 64);
        if word & bit == 0 {
            return 0..0;
        }
        if self.layout == Layout::PerNode {
            return node..node + 1;
        }
        let at = (self.rank[node / 64] + (word & (bit - 1)).count_ones()) as usize;
        self.offsets[at] as usize..self.offsets[at + 1] as usize
    }

    /// redo-fills pass 2: each row's key node, in the order the rows were written (both layouts; empty for a yes/no ask).
    #[must_use]
    pub fn keys(&self) -> &[u32] {
        &self.keys
    }

    /// The row number in the columns of a place `rows_of` gave.
    #[inline]
    #[must_use]
    pub fn row(&self, place: usize) -> usize {
        if self.layout == Layout::Keyed && !self.order.is_empty() { self.order[place] as usize } else { place }
    }
}

/// The answers to one file's asks, one table an ask in ask order; the buffers are kept from file to file.
#[derive(Clone, Debug, Default)]
pub struct FactColumns {
    /// The tables.
    pub tables: Vec<FactTable>,
}

/// The ids of the texts the provider writes the same in every file, by where the text is (memoised for the run), and of the
/// file's model names (memoised for the file).
#[derive(Debug, Default)]
struct TextMemo {
    fixed: HashMap<(usize, usize), u32, std::hash::BuildHasherDefault<crate::domain::model::names::types::KeyHasher>>,
    names: Vec<u32>,
}

/// redo-fills pass 2: a second text memo, for a caller whose text ids are not the walk's own caller's (the memo holds ids).
#[derive(Debug, Default)]
pub struct ColumnMemo(TextMemo);

impl ColumnMemo {
    /// A new file starts: its model names are forgotten (the fixed texts are kept for the run).
    pub fn new_file(&mut self) {
        self.0.names.clear();
    }
}

impl TextMemo {
    fn fixed(&mut self, ids: &mut dyn TextIds, text: NameText<'static>) -> u32 {
        *self.fixed.entry((text.0.as_ptr() as usize, text.0.len())).or_insert_with(|| ids.id(text.0.as_bytes()))
    }
    fn name(&mut self, ids: &mut dyn TextIds, names: &crate::domain::model::names::types::NameTable, code: NameCode) -> u32 {
        let at = usize::from(code);
        if let Some(found) = self.names.get(at)
            && *found != u32::MAX
        {
            return *found;
        }
        let id = match text_of(names, code) {
            Some(text) => ids.id(text.0.as_bytes()),
            None => ids.id(b""),
        };
        if at < 1 << 24 {
            if self.names.len() <= at {
                self.names.resize(at + 1, u32::MAX);
            }
            self.names[at] = id;
        }
        id
    }
}

/// The prototype walk: Python's walk, lending a file over both the reader (`fill`) and the direct columns (`columns`).
pub struct ColumnWalk {
    walk: PythonWalk<WalkPorts>,
    scratch: PythonWalkScratch,
    memo: TextMemo,
    words: FileWords,
    /// When set, the table is written `structured` with these enclosing kind sets.
    structure: Option<Vec<languages::KindSet>>,
}

/// The relations this walk writes per node as fact columns (`ColumnWalk::column`), by catalog name: a consumer may test
/// them per node.
pub const PER_NODE_FACTS: [&str; 9] =
    ["qualified-name", "identifier", "literal-properties", "numeric-literal", "string-literal", "expression-shape", "expression-type", "binding-event", "binding-read"];

/// File-local relations available through the walked file's complete fill.
pub const FILE_SET_FACTS: &[&str] = &[
    "static-import", "comment", "indent-region", "call-argument",
    "binding-use", "binding-shadow", "binding-resolution",
];

/// How a column's literal is spelled in the source as a word: whole, or its last segment after a separator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spelling {
    Whole,
    LastSegment(char),
}

/// The (relation, column) pairs whose text is spelled in the file as a name: a reference to `a.b.c` spells `c` (or imports
/// it under an alias by a statement that spells `c`); an identifier's and a definition's name are spelled whole.
pub const SPELLED_COLUMNS: [(&str, &str, Spelling); 4] = [
    ("qualified-name", "path", Spelling::LastSegment('.')),
    ("static-import", "target", Spelling::LastSegment('.')),
    ("identifier", "name", Spelling::Whole),
    ("defines", "name", Spelling::Whole),
];

/// The kind of the tokens that spell names (the words the first half's word index reads).
pub const NAME_KIND: &str = "name-token";

/// Whether `text` is a name as this language spells one (a Python identifier: a letter or `_`, then letters, digits or
/// `_`; Unicode letters and digits count).
#[must_use]
pub fn is_identifier(text: &str) -> bool {
    let mut cs = text.chars();
    cs.next().is_some_and(|c| c.is_alphabetic() || c == '_') && cs.all(|c| c.is_alphanumeric() || c == '_')
}

impl ColumnWalk {
    /// A walk under the default settings; `words` are the walk's fixed words (`walking::walk_words`), kept for the shapes' keys.
    #[must_use]
    pub fn new(words: FileWords) -> Self {
        // Spike (6f): `VEC_PY310` sets the pack's configured target; the default is the parser's (Latest).
        let settings = if std::env::var_os("VEC_PY310").is_some() {
            PythonParserSettings::new(crate::settings::PythonTargetVersion::Py310)
        } else {
            PythonParserSettings::default()
        };
        Self::with_settings(words, settings)
    }

    /// Writes facts for the same Python target that governs the source's syntax and annotation evaluation.
    #[must_use]
    pub fn with_settings(words: FileWords, settings: PythonParserSettings) -> Self {
        Self {
            walk: PythonWalk { ports: WalkPorts { settings, syntax: Arc::new(ruff_python_parser(settings)) } },
            scratch: PythonWalkScratch::default(),
            memo: TextMemo::default(),
            words,
            structure: None,
        }
    }

    /// Every table this walk writes from now on is written `structured` (`node_table::structured`) with the kind sets
    /// `enclosing`: its element sets and enclosing columns are written as the ids are numbered.
    pub fn structured(&mut self, enclosing: Vec<languages::KindSet>) {
        self.structure = Some(enclosing);
    }

    /// Walks one file into `table` and lends it to `visit`.
    ///
    /// # Errors
    ///
    /// When the file is not Python.
    pub fn walked(&mut self, path: &str, bytes: &[u8], table: &mut NodeTable, visit: &mut dyn FnMut(&mut ColumnFile<'_, '_>)) -> Result<(), WalkError> {
        let source_path = languages::SourcePath(path);
        if reads(source_path) != languages::Recognition::Reads {
            return Err(WalkError::Unsupported);
        }
        let kinds = <PythonWalk<WalkPorts> as languages::Walk>::KIND_COUNT;
        let mut builder = match &self.structure {
            Some(enclosing) => node_table::structured(table, kinds, enclosing),
            None => node_table::builder(table, kinds),
        };
        let target = self.walk.ports.settings.target_version().components();
        self.scratch.file.model.target = target;
        self.memo.names.clear();
        languages::facts::clear_words(&mut self.words);
        let mut built_parts = Built::default();
        let scratch = &mut self.scratch;
        let memo = &mut self.memo;
        let words = &mut self.words;
        let mut names = std::mem::take(&mut scratch.names);
        crate::domain::model::names::service::cleared(&mut names);
        self.walk.ports.syntax.walk(SourceText(bytes), source_kind(source_path), ReportedSyntax::Refusals, &mut builder, names, &mut |walked| match walked {
            Walked::Strings(tree) => {
                built_parts = Built::default();
                let mut hosts = Vec::new();
                if tree.strings.is_empty() {
                    return hosts;
                }
                let typed = TypedTree { segments: &tree.segments, parents: &tree.parents, ends: &tree.ends, nodes: &tree.nodes, annotations: tree.annotations, declarations: tree.declarations, named: &tree.names };
                form_imports(typed, &tree.imports, &mut scratch.forms);
                type_strings(
                    &mut FormSource { tree: typed, imports: &scratch.forms, model: &mut scratch.file.model, built: &mut built_parts, path: &mut scratch.path },
                    &tree.strings,
                    &mut hosts,
                );
                hosts
            }
            Walked::Whole(written, tree) => {
                lend(written, &tree, target, scratch, built_parts, memo, words, visit);
                // the walk's buffers go back to the worker for the next file
                scratch.names = tree.names;
                Vec::new()
            }
        });
        Ok(())
    }
}

#[expect(clippy::too_many_arguments, reason = "throwaway prototype")]
fn lend(
    builder: &mut languages::NodeTableBuilder<'_>,
    tree: &WalkedTree<'_>,
    target: crate::domain::standard_library::types::TargetVersion,
    scratch: &mut PythonWalkScratch,
    built_parts: Built,
    memo: &mut TextMemo,
    words: &mut FileWords,
    visit: &mut dyn FnMut(&mut ColumnFile<'_, '_>),
) {
    let table = node_table::finished(builder);
    let own = tree.segments.first().map_or(tree.nodes.len(), |segment| usize::from(segment.first));
    let mut file = PythonFile {
        syntax: SyntaxFile {
            table,
            nodes: tree.nodes.get(..own).unwrap_or_default(),
            tree: TypedTree { segments: &tree.segments, parents: &tree.parents, ends: &tree.ends, nodes: &tree.nodes, annotations: tree.annotations, declarations: tree.declarations, named: &tree.names },
            text: tree.text,
            file: FileId(0),
            completeness: tree.completeness,
        },
        target,
        model: &mut scratch.file.model,
        built: built_parts,
        analysed: Analysed::default(),
        buffers: &mut scratch.file.buffers,
    };
    visit(&mut ColumnFile { table, file: &mut file, relative_froms: &tree.relative_froms, memo, words });
}

/// Spike (6f probe): one node's evaluation region (`ColumnFile::evaluation_region`).
#[derive(Clone, Copy, Debug)]
pub struct Region {
    /// The node.
    pub id: u32,
    /// The root of the phase it is evaluated in.
    pub region: u32,
    /// That phase's kind: module, callable, generator or type.
    pub kind: &'static str,
    /// The annotation expression it is inside within its phase; `u32::MAX` for none.
    pub annotation: u32,
}

/// One walked file, lent over the reader and over the direct columns.
pub struct ColumnFile<'a, 'f> {
    /// The node table.
    pub table: &'a NodeTable,
    file: &'a mut PythonFile<'f>,
    relative_froms: &'a [Position],
    memo: &'a mut TextMemo,
    words: &'a mut FileWords,
}

/// The catalog place of a column of a relation.
macro_rules! place {
    ($relation:ident, $($column:tt)+) => {
        <languages::$relation as CatalogRelation>::columns()
            .iter()
            .position(|c| Column::$relation(*c) == Column::$relation(languages::$($column)+))
            .expect("catalog column")
    };
}

fn flag(value: Flag) -> u32 {
    u32::from(value == Flag::Raised)
}

mod resolution;
pub use resolution::BindingSite;

impl ColumnFile<'_, '_> {
    /// A source file without a relative from-import requests no package-relative alias.
    pub fn has_relative_imports(&self) -> bool {
        !self.relative_froms.is_empty()
    }

    /// Each relative import alias names a member (or, for a wildcard, just its source module).
    pub fn relative_imports(&self) -> impl Iterator<Item = (u32, u32, Option<&str>, Option<&str>)> {
        self.relative_froms.iter()
            .filter_map(|index| match self.file.syntax.nodes.get(usize::from(*index)) {
                Some(AnyNodeRef::StmtImportFrom(statement)) => Some(*statement),
                _ => None,
            })
            .flat_map(|statement| {
                statement.names.iter().filter_map(move |alias| {
                    let id = crate::domain::node::service::id_of(alias)?;
                    let name = alias.name.as_str();
                    Some((id.0, statement.level, statement.module.as_ref().map(|m| m.as_str()), (name != "*").then_some(name)))
                })
            })
    }

    /// The code the walk interned for each name token of the file's own tree, in id order, into `out` (cleared first): equal
    /// codes are equal texts. A code with the high bit set (`FIXED_TEXT`) is a fixed text of the language, the same text in
    /// every file of the run. Empty when the file's bytes were not text.
    pub fn name_token_codes(&self, out: &mut Vec<u32>) {
        out.clear();
        out.extend(self.file.syntax.tree.named.of_token.iter().map(|code| code.0));
    }

    /// redo-fills pass 2: the run's number of this file: the file every row of the file names (its file cells and the file
    /// part of its node ids) is this one, not `FileId(0)`.
    pub fn set_file(&mut self, file: FileId) {
        self.file.syntax.file = file;
    }

    /// The reader: today's rows of `relation` (the reference).
    pub fn fill(&mut self, relation: Relation, wanted: Wanted, words: &mut FileWords, out: &mut FilledColumns) -> FactAnswer {
        let mut source = fillers::file_source::<PythonWalk<WalkPorts>>(self.file);
        source.fill(relation, wanted, words, out)
    }

    /// Spike (ryo-fmx3.1, 6f probe): the provider's render, as the reader's file source answers it.
    pub fn render(&mut self, request: languages::RenderRequest) -> languages::RenderOutcome {
        let mut source = fillers::file_source::<PythonWalk<WalkPorts>>(self.file);
        source.render(request)
    }

    /// Spike (ryo-fmx3.1, 6f probe): the evaluation region of every node of the file, as the model's schedule walk assigns it:
    /// the phase the node is evaluated in (its root: the module, a callable, a generator body or a deferred type expression)
    /// and that phase's kind, and the annotation expression the node is inside within its phase (`u32::MAX`: none). The
    /// flag says whether native annotations are deferred in the file. `None` where the model is incomplete.
    pub fn evaluation_region(&mut self) -> Option<(bool, Vec<Region>)> {
        use crate::domain::model::lexical::service::parent_of;
        use crate::domain::model::lexical::types::AnnotationMark;
        use crate::domain::model::types::{BindingAnnotationTiming, BindingPhaseKind};
        let read = self.file.semantics(ModelPart::Flow);
        if read.model.flow.state == FlowState::Incomplete {
            return None;
        }
        let schedule = &read.model.schedule;
        let lexical = &read.model.lexical;
        let segments = read.tree.segments;
        let n = lexical.class.len();
        let mut region = vec![0u32; n];
        let mut kinds: HashMap<u32, &'static str> = HashMap::new();
        kinds.insert(0, "module");
        for phase in &schedule.phases {
            kinds.insert(
                phase.root.0,
                match phase.kind {
                    BindingPhaseKind::Callable => "callable",
                    BindingPhaseKind::Generator => "generator",
                    BindingPhaseKind::Type => "type",
                },
            );
        }
        for p in 0..n {
            let r = schedule.region.get(p).copied().unwrap_or(ryo::languages::Position(u32::MAX));
            region[p] = if r.0 != u32::MAX {
                r.0
            } else if p == 0 {
                0
            } else {
                let parent = parent_of(read.tree, ryo::languages::Position(p as u32)).0 as usize;
                if parent < p { region[parent] } else { 0 }
            };
        }
        let mut out = Vec::with_capacity(n);
        for p in 0..n {
            let root = region[p];
            let mut above = p as u32;
            let mut annotation = u32::MAX;
            loop {
                if entry(&lexical.annotation, ryo::languages::Position(above)) == AnnotationMark::Annotation {
                    annotation = table_id(segments, ryo::languages::Position(above)).0;
                    break;
                }
                if above == root || above == 0 {
                    break;
                }
                let parent = parent_of(read.tree, ryo::languages::Position(above)).0;
                if parent >= above {
                    break;
                }
                above = parent;
            }
            out.push(Region {
                id: table_id(segments, ryo::languages::Position(p as u32)).0,
                region: table_id(segments, ryo::languages::Position(root)).0,
                kind: kinds.get(&root).copied().unwrap_or("module"),
                annotation,
            });
        }
        // Nodes of the table the model does not number (tokens and identifiers under an expression) are evaluated where
        // their parent is.
        let elements = node_table::element_count(self.table).0 as usize;
        let mut by_id: Vec<Option<usize>> = vec![None; elements];
        for (row, r) in out.iter().enumerate() {
            if let Some(slot) = by_id.get_mut(r.id as usize) {
                *slot = Some(row);
            }
        }
        for id in 0..elements {
            if by_id[id].is_some() {
                continue;
            }
            let parent = node_table::parent_of(self.table, ryo::languages::Position(id as u32)).0 as usize;
            if parent < id
                && let Some(row) = by_id[parent]
            {
                let mut r = out[row];
                if r.annotation == u32::MAX && entry(&lexical.annotation, ryo::languages::Position(parent as u32)) == AnnotationMark::Annotation {
                    r.annotation = parent as u32;
                }
                r.id = id as u32;
                out.push(r);
                by_id[id] = Some(out.len() - 1);
            }
        }
        Some((schedule.annotations == BindingAnnotationTiming::Deferred, out))
    }

    /// Spike (redo-binding pass 4): the elements evaluated in a deferred type phase (the model's schedule: a phase of kind
    /// `Type`), as `evaluation_region` assigns regions, without the rest of its rows; tokens and identifiers the model does
    /// not number are where their parent is. `None` where the model is incomplete.
    pub fn type_regions(&mut self) -> Option<Vec<u32>> {
        use crate::domain::model::lexical::service::parent_of;
        use crate::domain::model::types::BindingPhaseKind;
        let read = self.file.semantics(ModelPart::Flow);
        if read.model.flow.state == FlowState::Incomplete {
            return None;
        }
        let schedule = &read.model.schedule;
        let segments = read.tree.segments;
        let n = read.model.lexical.class.len();
        let types: std::collections::HashSet<u32> =
            schedule.phases.iter().filter(|phase| matches!(phase.kind, BindingPhaseKind::Type)).map(|phase| phase.root.0).collect();
        if types.is_empty() {
            return Some(Vec::new());
        }
        let mut region = vec![0u32; n];
        let elements = node_table::element_count(self.table).0 as usize;
        let mut typed = vec![false; elements];
        let mut out = Vec::new();
        for p in 0..n {
            let r = schedule.region.get(p).copied().unwrap_or(ryo::languages::Position(u32::MAX));
            region[p] = if r.0 != u32::MAX {
                r.0
            } else if p == 0 {
                0
            } else {
                let parent = parent_of(read.tree, ryo::languages::Position(p as u32)).0 as usize;
                if parent < p { region[parent] } else { 0 }
            };
            if types.contains(&region[p]) {
                let id = table_id(segments, ryo::languages::Position(p as u32)).0;
                if let Some(slot) = typed.get_mut(id as usize) {
                    *slot = true;
                }
                out.push(id);
            }
        }
        let mut numbered = vec![false; elements];
        for p in 0..n {
            let id = table_id(segments, ryo::languages::Position(p as u32)).0 as usize;
            if let Some(slot) = numbered.get_mut(id) {
                *slot = true;
            }
        }
        for id in 0..elements {
            if numbered[id] {
                continue;
            }
            let parent = node_table::parent_of(self.table, ryo::languages::Position(id as u32)).0 as usize;
            if parent < id && numbered[parent] && typed[parent] {
                typed[id] = true;
                numbered[id] = true;
                out.push(id as u32);
            } else if parent < id && numbered[parent] {
                numbered[id] = true;
            }
        }
        Some(out)
    }

    /// Builds the parts of the semantic model the asks need, and no more (a file whose asks need none builds none): timed
    /// apart by the caller, so the split shows the model's build beside the writing of the columns.
    pub fn prepare(&mut self, asks: &[Ask]) {
        let mut parts = [false; 3];
        for ask in asks {
            match ask.relation {
                Relation::BindingEvent | Relation::BindingRead => parts[2] = true,
                Relation::QualifiedName if !ask.exists => parts[1] = true,
                Relation::QualifiedName => parts[0] = true,
                Relation::StringLiteral if !ask.exists => parts[0] = true,
                _ => {}
            }
        }
        for (wanted, part) in parts.into_iter().zip([ModelPart::Lexical, ModelPart::Resolution, ModelPart::Flow]) {
            if wanted {
                let _read = self.file.semantics(part);
            }
        }
    }

    /// The direct path: one table an ask, written into `out.tables` in ask order. An ask of a fact the prototype does not
    /// write leaves its table unanswered.
    pub fn columns(&mut self, asks: &[Ask], ids: &mut dyn TextIds, out: &mut FactColumns) {
        out.tables.resize_with(asks.len().max(out.tables.len()), FactTable::default);
        for (ask, table) in asks.iter().zip(&mut out.tables) {
            self.column(ask, ids, table);
        }
    }

    /// redo-fills pass 2: how many rows the reader's fill of `relation` writes in this file, where the file's own contents
    /// say it without the fill: a comment a comment token, an embedded expression a segment the walk wrote. None: not
    /// known without the fill.
    #[must_use]
    pub fn row_count(&self, relation: Relation) -> Option<usize> {
        match relation {
            Relation::Comment => Some(
                crate::domain::layout_facts::service::tokens(&self.file.syntax)
                    .filter(|at| at.token == crate::domain::tokens::types::Token::Comment)
                    .count(),
            ),
            Relation::EmbeddedExpression => Some(self.file.syntax.tree.segments.len()),
            // a row an import alias: a file with no alias has none
            Relation::StaticImport | Relation::StandardLibraryImport => nodes_of_kind(&self.file.syntax, Kind::Node(Node::Alias)).next().is_none().then_some(0),
            _ => None,
        }
    }

    /// `column` with the caller's own text memo (`ColumnMemo`): the ids are `ids`', never the walk's memo's.
    pub fn column_with(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, memo: &mut ColumnMemo) {
        std::mem::swap(self.memo, &mut memo.0);
        self.column(ask, ids, table);
        std::mem::swap(self.memo, &mut memo.0);
    }

    /// One ask's table.
    pub fn column(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable) {
        let elements = node_table::element_count(self.table).0 as usize;
        match ask.relation {
            Relation::QualifiedName => self.qualified_name(ask, ids, table, elements),
            Relation::Identifier => self.identifier(ask, ids, table, elements),
            Relation::LiteralProperties => self.literal_properties(ask, ids, table, elements),
            Relation::NumericLiteral => self.numeric_literal(ask, ids, table, elements),
            Relation::StringLiteral => self.string_literal(ask, ids, table, elements),
            Relation::ExpressionShape => self.expression_shape(ask, table, elements),
            Relation::ExpressionType => self.expression_type(ask, ids, table, elements),
            Relation::BindingEvent => self.binding_event(ask, ids, table, elements),
            Relation::BindingRead => self.binding_read(ask, ids, table, elements),
            _ => {
                table.begin(Layout::PerNode, 0, elements, &[], false, u64::MAX);
                table.answered = false;
            }
        }
    }

    fn qualified_name(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        let width = <languages::QualifiedName as CatalogRelation>::columns().len();
        let (path_at, provenance_at, major_at, minor_at) =
            (place!(QualifiedName, QualifiedNameColumn::Path), place!(QualifiedName, QualifiedNameColumn::Provenance), place!(QualifiedName, QualifiedNameColumn::SinceMajor), place!(QualifiedName, QualifiedNameColumn::SinceMinor));
                table.begin(Layout::PerNode, width, elements, &[path_at, provenance_at, major_at, minor_at], ask.exists, ask.columns);
        let read = self.file.semantics(if ask.exists { ModelPart::Lexical } else { ModelPart::Resolution });
        let memo = &mut *self.memo;
        let labels = [refinements::IMPORT, BUILTIN, LOCAL, UNRESOLVED, AMBIGUOUS].map(|text| memo.fixed(ids, NameText(text)));
        let empty = ids.id(b"");
        let mut path = QualifiedPath::default();
        let lexical = &read.model.lexical;
        for at in 0..u32::try_from(lexical.class.len()).unwrap_or(u32::MAX) {
            let at = Position(at);
            if !matches!(entry(&lexical.class, at), NodeClass::Name | NodeClass::Attribute) || entry(&lexical.usage, at) != NameUse::Load {
                continue;
            }
            let node = table_id(read.tree.segments, at).0;
            if ask.exists {
                table.mark(node);
                table.rows += 1;
                continue;
            }
            let provenance = qualify(read.model, at, &mut path);
            let (label, since) = match provenance {
                NameProvenance::Import => (labels[0], NOT_APPLICABLE),
                NameProvenance::Builtin(since) => (labels[1], since),
                NameProvenance::Local => (labels[2], NOT_APPLICABLE),
                NameProvenance::Unresolved => (labels[3], NOT_APPLICABLE),
                NameProvenance::Ambiguous => (labels[4], NOT_APPLICABLE),
            };
            let has_path = matches!(provenance, NameProvenance::Import | NameProvenance::Builtin(_));
            let text = if has_path { ids.id(path.0.as_bytes()) } else { empty };
            table.put(node, &[(path_at, text), (provenance_at, label), (major_at, since.major.0 as u32), (minor_at, since.minor.0 as u32)]);
        }
    }

    fn identifier(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        let width = <languages::Identifier as CatalogRelation>::columns().len();
        let name_at = place!(Identifier, IdentifierColumn::Name);
        table.begin(Layout::PerNode, width, elements, &[name_at], ask.exists, ask.columns);
        let syntax = &self.file.syntax;
        if syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            table.answered = false;
            return;
        }
        fn name_of<'n>(node: AnyNodeRef<'n>) -> Option<&'n str> {
            let name = match node {
                AnyNodeRef::ExprName(name) => name.id.as_str(),
                AnyNodeRef::Identifier(identifier) => identifier.id.as_str(),
                _ => return None,
            };
            (!name.is_empty() && name != crate::domain::name_facts::constants::EVERYTHING).then_some(name)
        }
        for kind in [Kind::Node(Node::Name), Kind::Extra(TypedExtra::Identifier)] {
            for Numbered { id, node } in nodes_of_kind(syntax, kind) {
                let Some(name) = name_of(node) else { continue };
                if ask.exists {
                    table.mark(id.0);
                    table.rows += 1;
                } else {
                    table.put(id.0, &[(name_at, ids.id(name.as_bytes()))]);
                }
            }
        }
        let tree = syntax.tree;
        let first = tree.segments.first().map_or(tree.nodes.len(), |segment| usize::from(segment.first));
        for (place, node) in tree.nodes.iter().enumerate().skip(first) {
            let Some(name) = name_of(*node) else { continue };
            let id = table_id(tree.segments, Position(u32::try_from(place).unwrap_or(u32::MAX))).0;
            if ask.exists {
                table.mark(id);
                table.rows += 1;
            } else {
                table.put(id, &[(name_at, ids.id(name.as_bytes()))]);
            }
        }
    }

    fn literal_properties(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        let width = <languages::LiteralProperties as CatalogRelation>::columns().len();
        let (kind_at, zero_at, empty_at, boolean_at) = (
            place!(LiteralProperties, LiteralPropertiesColumn::Kind),
            place!(LiteralProperties, LiteralPropertiesColumn::Zero),
            place!(LiteralProperties, LiteralPropertiesColumn::Empty),
            place!(LiteralProperties, LiteralPropertiesColumn::Boolean),
        );
        table.begin(Layout::PerNode, width, elements, &[kind_at, zero_at, empty_at, boolean_at], ask.exists, ask.columns);
        for Numbered { id, node } in nodes_of_kind(&self.file.syntax, Kind::Node(Node::Constant)) {
            if let Some(found) = properties(node) {
                if ask.exists {
                    table.mark(id.0);
                    table.rows += 1;
                    continue;
                }
                let kind = self.memo.fixed(ids, found.kind);
                table.put(id.0, &[(kind_at, kind), (zero_at, flag(found.zero)), (empty_at, flag(found.empty)), (boolean_at, flag(found.boolean))]);
            }
        }
    }

    fn numeric_literal(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        let width = <languages::NumericLiteral as CatalogRelation>::columns().len();
        let (kind_at, value_at, exact_at) =
            (place!(NumericLiteral, NumericLiteralColumn::Kind), place!(NumericLiteral, NumericLiteralColumn::Value), place!(NumericLiteral, NumericLiteralColumn::ExactDigits));
        table.begin(Layout::PerNode, width, elements, &[kind_at, value_at, exact_at], ask.exists, ask.columns);
        let value = &mut self.file.buffers.text;
        for Numbered { id, .. } in nodes_of_kind(&self.file.syntax, Kind::Node(Node::Constant)) {
            if let Some(found) = number(&self.file.syntax, id, value) {
                if ask.exists {
                    table.mark(id.0);
                    table.rows += 1;
                    continue;
                }
                let kind = self.memo.fixed(ids, found.kind);
                let text = if table.wants(value_at) { ids.id(value.0.as_bytes()) } else { 0 };
                table.put(id.0, &[(kind_at, kind), (value_at, text), (exact_at, flag(found.exact))]);
            }
        }
    }

    fn string_literal(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        let width = <languages::StringLiteral as CatalogRelation>::columns().len();
        let (owner_at, value_at, import_at) =
            (place!(StringLiteral, StringLiteralColumn::Owner), place!(StringLiteral, StringLiteralColumn::Value), place!(StringLiteral, StringLiteralColumn::Import));
        table.begin(Layout::PerNode, width, elements, &[owner_at, value_at, import_at], ask.exists, ask.columns);
        if !ask.exists {
            built(self.file.model, &mut self.file.built, ModelPart::Lexical, self.file.syntax.tree);
        }
        let empty = ids.id(b"");
        let owned = &self.file.model.lexical.scope_of;
        let joined = &mut self.file.buffers.text;
        for kind in [Node::Constant, Node::JoinedStr, Node::TemplateStr] {
            for Numbered { id, node } in nodes_of_kind(&self.file.syntax, Kind::Node(kind)) {
                let value = match string_value(node, joined) {
                    Some(StringValue::Known(text)) => {
                        if ask.exists || !table.wants(value_at) {
                            0
                        } else {
                            ids.id(text.0.as_bytes())
                        }
                    }
                    Some(StringValue::Interpolated) => empty,
                    None => continue,
                };
                if ask.exists {
                    table.mark(id.0);
                    table.rows += 1;
                    continue;
                }
                let owner = owned.get(usize::from(id)).copied().unwrap_or(Position(0)).0;
                table.put(id.0, &[(owner_at, owner), (value_at, value), (import_at, flag(loaded(&self.file.syntax, id)))]);
            }
        }
    }

    /// `expression-shape`: the yes/no question needs no shape at all, only whether every node below the expression has one,
    /// which is one pass from the last node to the first with no key and no hash; whole, it is the reader's class index.
    fn expression_shape(&mut self, ask: &Ask, table: &mut FactTable, elements: usize) {
        let width = <languages::ExpressionShape as CatalogRelation>::columns().len();
        let shape_at = place!(ExpressionShape, ExpressionShapeColumn::Shape);
        table.begin(Layout::PerNode, width, elements, &[shape_at], ask.exists, ask.columns);
        let syntax = &self.file.syntax;
        if syntax.completeness.syntax == languages::SyntaxCompleteness::Incomplete {
            table.answered = false;
            return;
        }
        let nodes = node_table::node_count(syntax.table).0;
        let count = syntax.nodes.len();
        let is_expression = |id: Position| category_of(kind_of(node_table::kind_of(syntax.table, id))) == Category::Expression;
        if ask.exists {
            // whole[id]: the node and every node below it have a shape (a node child is after its parent in id order)
            let mut whole = vec![false; count];
            let ipy = crate::domain::syntax::service::kind_bits(Kind::Extra(TypedExtra::IpyEscapeCommand));
            for id in (0..count).rev() {
                let at = Position(id as u32);
                let ok = node_table::kind_of(syntax.table, at) != ipy
                    && node_table::children_of(syntax.table, at).iter().all(|child| usize::from(*child) >= nodes || whole.get(usize::from(*child)).copied().unwrap_or(false));
                whole[id] = ok;
                if ok && is_expression(at) {
                    table.mark(at.0);
                    table.rows += 1;
                }
            }
            return;
        }
        if self.file.analysed.shapes == PartState::Pending {
            shapes(&self.file.syntax, &mut ShapeBuild { index: &mut self.file.buffers.shapes, words: self.words, text: &mut self.file.buffers.text });
            self.file.analysed.shapes = PartState::Ready;
        }
        let syntax = &self.file.syntax;
        for (id, class) in self.file.buffers.shapes.classes.iter().enumerate().take(count) {
            let at = Position(id as u32);
            if let Some(class) = class
                && category_of(kind_of(node_table::kind_of(syntax.table, at))) == Category::Expression
            {
                table.put(at.0, &[(shape_at, class.0)]);
            }
        }
    }

    fn expression_type(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        let width = <languages::ExpressionType as CatalogRelation>::columns().len();
        let (resolution_at, type_at) = (place!(ExpressionType, ExpressionTypeColumn::Resolution), place!(ExpressionType, ExpressionTypeColumn::Type));
        table.begin(Layout::Keyed, width, elements, &[], false, ask.columns);
        if self.file.analysed.types == PartState::Pending {
            resolutions(&self.file.syntax, &mut self.file.buffers.types);
            self.file.analysed.types = PartState::Ready;
        }
        let syntax = &self.file.syntax;
        let complete = syntax.completeness.syntax != languages::SyntaxCompleteness::Incomplete;
        let memo = &mut *self.memo;
        let name = crate::domain::expression_types::constants::name;
        let [atom, union, unknown, type_error, none] =
            [type_labels::ATOM, type_labels::UNION, type_labels::UNKNOWN, type_labels::TYPE_ERROR, type_labels::NO_TYPE].map(|text| memo.fixed(ids, NameText(text)));
        for (place, found) in self.file.buffers.types.iter().enumerate() {
            let id = table_id(syntax.tree.segments, Position(u32::try_from(place).unwrap_or(u32::MAX)));
            if category_of(kind_of(node_table::kind_of(syntax.table, id))) != Category::Expression {
                continue;
            }
            let write = |table: &mut FactTable, resolution: u32, kind: u32| {
                if ask.exists {
                    table.mark(id.0);
                    table.keys.push(id.0);
                    table.rows += 1;
                } else {
                    table.push(id.0, &[(resolution_at, resolution), (type_at, kind)]);
                }
            };
            match found {
                Resolution::Atom(kind) => write(table, atom, memo.fixed(ids, name(*kind))),
                Resolution::Union(kinds) => {
                    for kind in kinds {
                        write(table, union, memo.fixed(ids, name(*kind)));
                    }
                }
                Resolution::Unknown => write(table, unknown, none),
                Resolution::TypeError => write(table, type_error, none),
            }
        }
        if ask.exists {
            table.layout = Layout::PerNode;
        }
        table.finish(elements);
        table.answered = complete;
    }

    fn binding_event(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        use crate::domain::binding_facts::service::event_word;
        let width = <languages::BindingEvent as CatalogRelation>::columns().len();
        let cols = (
            place!(BindingEvent, BindingEventColumn::Occurrence),
            place!(BindingEvent, BindingEventColumn::Name),
            place!(BindingEvent, BindingEventColumn::Scope),
            place!(BindingEvent, BindingEventColumn::Phase),
            place!(BindingEvent, BindingEventColumn::Kind),
            place!(BindingEvent, BindingEventColumn::Ordinal),
            place!(BindingEvent, BindingEventColumn::Lexical),
        );
        table.begin(Layout::Keyed, width, elements, &[], false, ask.columns);
        let read = self.file.semantics(ModelPart::Flow);
        if read.model.flow.state == FlowState::Incomplete {
            table.answered = false;
            table.finish(elements);
            return;
        }
        let memo = &mut *self.memo;
        let by_scope = ask.key == cols.2;
        let segments = read.tree.segments;
        for event in &read.model.flow.events {
            let occurrence = table_id(segments, event.at).0;
            let scope = table_id(segments, event.scope).0;
            let key = if by_scope { scope } else { occurrence };
            if ask.exists {
                table.mark(key);
                table.keys.push(key);
                table.rows += 1;
                continue;
            }
            let name = if table.wants(cols.1) { memo.name(ids, &read.model.names, event.name) } else { 0 };
            let kind = memo.fixed(ids, event_word(event.kind));
            table.push(
                key,
                &[
                    (cols.0, occurrence),
                    (cols.1, name),
                    (cols.2, scope),
                    (cols.3, table_id(segments, event.phase).0),
                    (cols.4, kind),
                    (cols.5, event.ordinal.0 as u32),
                    (cols.6, table_id(segments, event.lexical).0),
                ],
            );
        }
        if ask.exists {
            table.layout = Layout::PerNode;
        }
        table.finish(elements);
    }

    fn binding_read(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        use crate::domain::binding_facts::service::{context_word, state_word};
        use crate::domain::model::reaching::types::FlowReadState;
        let width = <languages::BindingRead as CatalogRelation>::columns().len();
        let cols = (
            place!(BindingRead, BindingReadColumn::Name),
            place!(BindingRead, BindingReadColumn::Scope),
            place!(BindingRead, BindingReadColumn::Phase),
            place!(BindingRead, BindingReadColumn::Context),
            place!(BindingRead, BindingReadColumn::State),
            place!(BindingRead, BindingReadColumn::SinceMajor),
            place!(BindingRead, BindingReadColumn::SinceMinor),
        );
        table.begin(Layout::Keyed, width, elements, &[], false, ask.columns);
        let read = self.file.semantics(ModelPart::Flow);
        if read.model.flow.state == FlowState::Incomplete {
            table.answered = false;
            table.finish(elements);
            return;
        }
        let memo = &mut *self.memo;
        let segments = read.tree.segments;
        for each in &read.model.flow.reads {
            let key = table_id(segments, each.at).0;
            if ask.exists {
                table.mark(key);
                table.keys.push(key);
                table.rows += 1;
                continue;
            }
            let since = match each.state {
                FlowReadState::Builtin(since) | FlowReadState::DeclarationOrBuiltin(since) => since,
                FlowReadState::Unique | FlowReadState::Alternatives | FlowReadState::PossiblyUnbound | FlowReadState::Unbound | FlowReadState::Uncertain => NOT_APPLICABLE,
            };
            let name = if table.wants(cols.0) { memo.name(ids, &read.model.names, each.name) } else { 0 };
            let context = memo.fixed(ids, context_word(each.context));
            let state = memo.fixed(ids, state_word(each.state));
            table.push(
                key,
                &[
                    (cols.0, name),
                    (cols.1, table_id(segments, each.scope).0),
                    (cols.2, table_id(segments, each.phase).0),
                    (cols.3, context),
                    (cols.4, state),
                    (cols.5, since.major.0 as u32),
                    (cols.6, since.minor.0 as u32),
                ],
            );
        }
        if ask.exists {
            table.layout = Layout::PerNode;
        }
        table.finish(elements);
    }
}
