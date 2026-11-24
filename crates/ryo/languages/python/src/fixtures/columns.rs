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
use crate::domain::literal_expression_parsing::types::LiteralParseOutcome;
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
pub use ryo::languages::contract::columns::{Ask, FactColumns, FactTable, Layout, TextIds};

/// The ids of the texts the provider writes the same in every file, by where the text is (memoised for the run), and of the
/// file's model names (memoised for the file).
#[derive(Debug, Default)]
struct TextMemo {
    fixed: HashMap<(usize, usize), u32, std::hash::BuildHasherDefault<crate::domain::model::names::types::KeyHasher>>,
    literals: HashMap<Box<str>, LiteralParseOutcome>,
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
    /// The grammar result depends on the exact source spelling. Keep short
    /// spellings for this run. Each occurrence still checks its source tokens.
    fn literal_outcome(&mut self, spelling: &str, parse: impl FnOnce() -> LiteralParseOutcome) -> LiteralParseOutcome {
        if spelling.len() > crate::domain::grammar_cache::constants::KEY_CAPACITY {
            return parse();
        }
        if let Some(&outcome) = self.literals.get(spelling) {
            return outcome;
        }
        let outcome = parse();
        self.literals.insert(spelling.into(), outcome);
        outcome
    }

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
    /// When set, the table is written `structured` with these enclosing kind sets.
    structure: Option<Vec<languages::KindSet>>,
    native_navigation:bool,
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
        let mut scratch = PythonWalkScratch::default();
        scratch.names.shapes = Some(Box::new(crate::domain::shape_facts::walk::WalkShapes::new(words)));
        Self {
            walk: PythonWalk { ports: WalkPorts { settings, syntax: Arc::new(ruff_python_parser(settings)) } },
            scratch,
            memo: TextMemo::default(),
            structure: None,
            native_navigation:false,
        }
    }

    /// Every table this walk writes from now on is written `structured` (`node_table::structured`) with the kind sets
    /// `enclosing`: its element sets and enclosing columns are written as the ids are numbered.
    pub fn structured(&mut self, enclosing: Vec<languages::KindSet>) {
        self.structure = Some(enclosing);
    }

    /// Select the file walk for consumers reading token text from source spans.
    /// Semantic answers are built from the retained typed tree on first demand.
    pub fn defer_lexical(&mut self) {
        self.scratch.names.deferred = true;
    }

    /// Record typed-node and token intervals for the summary walk. Readers
    /// materialize ordered child slices on their first ordinary slice request.
    pub fn use_native_navigation(&mut self){self.native_navigation=true;}

    /// Walk one file into the table and lend it to the callback.
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
        if self.native_navigation {
            node_table::record_token_ranges(&mut builder);
            node_table::defer_child_ids(&mut builder);
        }
        let target = self.walk.ports.settings.target_version().components();
        self.scratch.file.model.target = target;
        self.memo.names.clear();
        let mut built_parts = Built::default();
        let scratch = &mut self.scratch;
        let memo = &mut self.memo;
        let mut names = std::mem::take(&mut scratch.names);
        if let Some(shapes) = &mut names.shapes { shapes.clear(); }
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
                lend(written, &tree, target, scratch, built_parts, memo, visit);
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
    let shapes = tree.names.shapes.as_deref().expect("the column walk requests expression classes");
    let mut empty = Vec::new();
    if tree.completeness.syntax == languages::SyntaxCompleteness::Complete && tree.completeness.lexical == languages::SyntaxCompleteness::Complete {
        let absent = |kind| node_table::ids_of_kind(table, crate::domain::syntax::service::kind_bits(kind)).is_empty();
        if !shapes.carriers.string {
            empty.extend(["literal-expression-parse", "raw-format-segment", "raw-format-field"]);
        }
        if !shapes.carriers.bytes { empty.push("bytes-literal"); }
        if !shapes.carriers.number { empty.push("numeric-literal"); }
        let text = shapes.carriers.string || !absent(Kind::Node(Node::JoinedStr)) || !absent(Kind::Node(Node::TemplateStr));
        if !text {
            empty.extend(["string-literal", "brace-format-parse", "brace-format-field", "format-specification"]);
            if !shapes.carriers.bytes { empty.extend(["format-parse", "format-field"]); }
        }
        if absent(Kind::Node(Node::JoinedStr)) {
            empty.push("interpolated-string-literal");
            // A replacement field needs its dialect's opening delimiter in
            // decoded text. A string with escapes can differ from its token.
            if text && absent(Kind::Node(Node::TemplateStr)) {
                if !shapes.carriers.percent && !shapes.carriers.bytes { empty.push("format-field"); }
                if !shapes.carriers.brace { empty.extend(["brace-format-field", "format-specification"]); }
            }
        }
        if absent(Kind::Node(Node::Arg)) { empty.push("parameter"); }
        if absent(Kind::Node(Node::ClassDef)) { empty.push("inherits"); }
        if absent(Kind::Node(Node::FunctionDef)) && absent(Kind::Node(Node::AsyncFunctionDef)) {
            empty.extend(["function", "function-terminal"]);
        }
        if absent(Kind::Node(Node::Call)) { empty.push("call-argument"); }
        if absent(Kind::Extra(TypedExtra::Decorator)) { empty.push("decorates"); }
        if absent(Kind::Node(Node::Alias)) { empty.extend(["static-import", "standard-library-import"]); }
        if absent(Kind::Token(crate::domain::tokens::types::Token::Comment)) { empty.push("comment"); }
        if tree.segments.is_empty() { empty.push("embedded-expression"); }
    }
    if node_table::syntax_error_count(table).0 == 0 { empty.push("syntax-error"); }
    let immediate_annotations = target < crate::domain::binding_schedule::constants::LAZY_ANNOTATIONS_FROM
        && tree.annotations == crate::domain::model::types::BindingAnnotationTiming::Immediate;
    visit(&mut ColumnFile { table, empty, immediate_annotations, shapes, import_presence: tree.import_presence, file: &mut file, relative_froms: &tree.relative_froms, memo });
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
    /// Catalog relations the completed parser walk certifies to have zero rows.
    pub empty: Vec<&'static str>,
    /// The walk's compiler features and target give annotations immediate evaluation.
    pub immediate_annotations: bool,
    shapes: &'a crate::domain::shape_facts::walk::WalkShapes,
    /// The node table.
    pub table: &'a NodeTable,
    file: &'a mut PythonFile<'f>,
    relative_froms: &'a [Position],
    import_presence: crate::domain::node::types::ImportPresence,
    memo: &'a mut TextMemo,
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

mod forward_reads;
mod expression_parts;
pub use expression_parts::CallSite;
mod assignment;
mod imports;
mod lexical_lines;
pub use imports::ImportAlias;
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
    /// every file of the run. Empty when the file's bytes were not text or the caller deferred lexical collection.
    pub fn name_token_codes(&self, out: &mut Vec<u32>) {
        out.clear();
        out.extend(self.file.syntax.tree.named.of_token.iter().map(|code| code.0));
    }

    /// Literal identifier spellings in the declared columns occur in these source bytes.
    /// Decoded embedded trees and Unicode identifier normalization may spell a word differently.
    #[must_use]
    pub fn plain_spellings(&self) -> bool {
        let syntax = &self.file.syntax;
        syntax.completeness.syntax == languages::SyntaxCompleteness::Complete
            && syntax.completeness.lexical == languages::SyntaxCompleteness::Complete
            && syntax.tree.segments.is_empty()
            && syntax.text.0.is_ascii()
    }

    /// redo-fills pass 2: the run's number of this file: the file every row of the file names (its file cells and the file
    /// part of its node ids) is this one, not `FileId(0)`.
    pub fn set_file(&mut self, file: FileId) {
        self.file.syntax.file = file;
    }

    /// redo-fills pass 3: builds the model's part for the asked families once, before any family reads it: `flow` (the
    /// lexical part and the schedule interpreted), else `resolution` (the lexical part and the static definitions). A
    /// part already built is not built again.
    pub fn build_model(&mut self, flow: bool, resolution: bool) {
        if flow {
            let _read = self.file.semantics(ModelPart::Flow);
        }
        if resolution {
            let _read = self.file.semantics(ModelPart::Resolution);
        }
    }

    /// The reader: today's rows of `relation` (the reference).
    pub fn fill(&mut self, relation: Relation, wanted: Wanted, words: &mut FileWords, out: &mut FilledColumns) -> FactAnswer {
        let mut source = fillers::file_source::<PythonWalk<WalkPorts>>(self.file);
        source.fill(relation, wanted, words, out)
    }

    /// Write only the requested columns, retaining cardinality for presence-only readers.
    pub fn fill_columns(&mut self, relation: Relation, mask: &languages::ColumnProjection, words: &mut FileWords, out: &mut FilledColumns) -> (FactAnswer, usize) {
        fillers::file_source::<PythonWalk<WalkPorts>>(self.file).fill_columns(relation, Wanted::Whole, mask, words, out)
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
            Relation::Comment => Some(crate::domain::layout_facts::service::token_ids(&self.file.syntax, crate::domain::tokens::types::Token::Comment).len()),
            Relation::EmbeddedExpression => Some(self.file.syntax.tree.segments.len()),
            // a row an import alias: a file with no alias has none
            Relation::StaticImport | Relation::StandardLibraryImport => nodes_of_kind(&self.file.syntax, Kind::Node(Node::Alias)).next().is_none().then_some(0),
            _ => None,
        }
    }

    /// Existence of a complete relation without writing other cells of its rows.
    pub fn summary_presence(&mut self,relation:Relation)->Option<bool> {
        match relation {
            Relation::BindingEvent|Relation::BindingRead=>{
                let read=self.file.semantics(ModelPart::Flow);
                Some(read.model.flow.state==FlowState::Incomplete || if relation==Relation::BindingEvent {
                    !read.model.flow.events.is_empty()
                }else{!read.model.flow.reads.is_empty()})
            }
            Relation::LiteralProperties=>Some(nodes_of_kind(&self.file.syntax,Kind::Node(Node::Constant)).any(|v|properties(v.node).is_some())),
            _=>self.projected_keys(relation).map(|mut keys|keys.next().is_some())
        }
    }
    /// Complete keys for the shape relation when every native node has a shape.
    /// Escape commands and their ancestors can lack a class, so a file containing
    /// one uses the ordinary shape reader. Embedded segments have separate ids.
    #[must_use]
    pub fn projected_keys(&self, relation: Relation) -> Option<impl Iterator<Item = Position> + '_> {
        let syntax=&self.file.syntax;
        let escape=crate::domain::syntax::service::kind_bits(Kind::Extra(TypedExtra::IpyEscapeCommand));
        if relation!=Relation::ExpressionShape || syntax.completeness.syntax==languages::SyntaxCompleteness::Incomplete
            || !syntax.tree.segments.is_empty() || !node_table::ids_of_kind(syntax.table,escape).is_empty() { return None; }
        Some((0..syntax.nodes.len()).map(|i|Position(i as u32)).filter(|&p|
            category_of(kind_of(node_table::kind_of(syntax.table,p)))==Category::Expression))
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
            Relation::LiteralExpressionParse => self.literal_parse(ask, ids, table, elements),
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
        let qualify_value = !ask.exists && [path_at, provenance_at, major_at, minor_at].iter().any(|c| table.wants(*c));
        let read = self.file.semantics(if qualify_value { ModelPart::Resolution } else { ModelPart::Lexical });
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
            if !qualify_value { table.put(node, &[]); continue; }
            let provenance = qualify(read.model, at, &mut path);
            let (label, since) = match provenance {
                NameProvenance::Import => (labels[0], NOT_APPLICABLE),
                NameProvenance::Builtin(since) => (labels[1], since),
                NameProvenance::Local => (labels[2], NOT_APPLICABLE),
                NameProvenance::Unresolved => (labels[3], NOT_APPLICABLE),
                NameProvenance::Ambiguous => (labels[4], NOT_APPLICABLE),
            };
            let has_path = matches!(provenance, NameProvenance::Import | NameProvenance::Builtin(_));
            let text = if has_path && table.wants(path_at) { ids.id(path.0.as_bytes()) } else { empty };
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
        let wants_name = !ask.exists && table.wants(name_at);
        let mut emit = |id: Position, node: AnyNodeRef<'_>| {
            let name = match node {
                AnyNodeRef::ExprName(name) => name.id.as_str(),
                AnyNodeRef::Identifier(identifier) => identifier.id.as_str(),
                _ => return,
            };
            if name.is_empty() || name == crate::domain::name_facts::constants::EVERYTHING { return; }
            if ask.exists {
                table.mark(id.0);
                table.rows += 1;
            } else {
                let word = if wants_name { ids.id(name.as_bytes()) } else { 0 };
                table.put(id.0, &[(name_at, word)]);
            }
        };
        for kind in [Kind::Node(Node::Name), Kind::Extra(TypedExtra::Identifier)] {
            for Numbered { id, node } in nodes_of_kind(syntax, kind) {
                emit(id, node);
            }
        }
        let tree = syntax.tree;
        let first = tree.segments.first().map_or(tree.nodes.len(), |segment| usize::from(segment.first));
        for (place, node) in tree.nodes.iter().enumerate().skip(first) {
            emit(table_id(tree.segments, Position(place as u32)), *node);
        }
    }

    fn literal_parse(&mut self, ask: &Ask, ids: &mut dyn TextIds, table: &mut FactTable, elements: usize) {
        use crate::domain::literal_expression_parsing::service::outcome_name;
        use crate::inbound::contracts::parse::readers::literal_expression::{composition::available, constants::PYTHON_STRING_ANNOTATION};
        use ruff_text_size::Ranged as _;

        let dialect = place!(LiteralExpressionParse, LiteralExpressionParseColumn::Dialect);
        let outcome = place!(LiteralExpressionParse, LiteralExpressionParseColumn::Outcome);
        let width = languages::LiteralExpressionParse::columns().len();
        table.begin(Layout::PerNode, width, elements, &[dialect, outcome], ask.exists, ask.columns);
        let syntax = &self.file.syntax;
        if [syntax.completeness.syntax, syntax.completeness.lexical].contains(&languages::SyntaxCompleteness::Incomplete) {
            table.answered = false;
            return;
        }
        let dialect_id = if !ask.exists && table.wants(dialect) { self.memo.fixed(ids, NameText(PYTHON_STRING_ANNOTATION)) } else { 0 };
        let mut tokens = Vec::new();
        for Numbered { id, node } in nodes_of_kind(syntax, Kind::Node(Node::Constant)) {
            let AnyNodeRef::ExprStringLiteral(literal) = node else { continue };
            let spelling = syntax.text.0.get(literal.range().start().to_usize()..literal.range().end().to_usize());
            let Some(spelling) = spelling else { table.answered = false; return; };
            if !available(syntax, node, &mut tokens) { table.answered = false; return; }
            if ask.exists {
                table.mark(id.0);
                table.rows += 1;
                continue;
            }
            let outcome_id = if table.wants(outcome) {
                let parsed = self.memo.literal_outcome(spelling, || crate::outbound::literal_expression::adapters::ruff_string_annotation::grammar::parsed_in_place(literal, syntax.text.0));
                self.memo.fixed(ids, outcome_name(parsed))
            } else { 0 };
            table.put(id.0, &[(dialect, dialect_id), (outcome, outcome_id)]);
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
                let kind = if table.wants(kind_at) { self.memo.fixed(ids, found.kind) } else { 0 };
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
                let kind = if table.wants(kind_at) { self.memo.fixed(ids, found.kind) } else { 0 };
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
        if !ask.exists && table.wants(owner_at) {
            built(self.file.model, &mut self.file.built, ModelPart::Lexical, self.file.syntax.tree);
        }
        let empty = ids.id(b"");
        let owned = &self.file.model.lexical.scope_of;
        let joined = &mut self.file.buffers.text;
        for kind in [Node::Constant, Node::JoinedStr, Node::TemplateStr] {
            for Numbered { id, node } in nodes_of_kind(&self.file.syntax, Kind::Node(kind)) {
                let value = if ask.exists || !table.wants(value_at) {
                    if !matches!(node, AnyNodeRef::ExprStringLiteral(_) | AnyNodeRef::ExprFString(_) | AnyNodeRef::ExprTString(_)) { continue; }
                    0
                } else {
                    match string_value(node, joined) {
                        Some(StringValue::Known(text)) => ids.id(text.0.as_bytes()),
                        Some(StringValue::Interpolated) => empty,
                        None => continue,
                    }
                };
                if ask.exists {
                    table.mark(id.0);
                    table.rows += 1;
                    continue;
                }
                let owner = owned.get(usize::from(id)).copied().unwrap_or(Position(0)).0;
                let import = if table.wants(import_at) { flag(loaded(&self.file.syntax, id)) } else { 0 };
                table.put(id.0, &[(owner_at, owner), (value_at, value), (import_at, import)]);
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
        for (at, class) in self.shapes.rows() {
            if ask.exists {
                table.mark(at.0);
                table.rows += 1;
            } else {
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
