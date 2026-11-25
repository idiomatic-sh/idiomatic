//! The `lexical_facts` service: free functions named by verb that write the rows of six relations from the model, with no IO and
//! no ports. Each writes every row of its relation; the per-file view is whatever implements `ModelView`.
//!
//! `resolves` and `qualified-name` keep their present meaning: one static answer per name that is read (the last definition
//! before it in source order, else the first in a function-like scope; `definitions::service` states the rule). Whether they
//! become the reaching set with the selection in the rule pack is an open decision of the product's owner.

use super::types::{NameProvenance, ProvenanceWords, QualifiedPath};
use crate::domain::definitions::constants as refinements;
use crate::domain::model::definitions::types::{DefinitionWord, NameDefinition};
use crate::domain::model::resolution::service::{proof_of, resolved_at};
use crate::domain::model::resolution::types::OriginProof;
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::syntax::constants::DOT;
use crate::domain::model::lexical::service::{entry, name_of, parent_of, scope_at, scope_form};
use crate::domain::model::lexical::types::{ImportOrigin, ImportRename, NameUse, NodeClass, ScopeForm};
use crate::domain::model::names::service::{builtin_of, fixed_word, name_word, text_of, word};
use crate::domain::model::names::types::{FactWords, NameCode, NameTable};
use crate::domain::model::view::types::{ModelPart, ModelRead, ModelView, SemanticModel};
use crate::domain::node::service::table_id;
use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::qualification::constants::{
    AMBIGUOUS, BUILTIN, BUILTIN_MODULE, BUILTIN_NAMES, LOCAL, NOT_APPLICABLE, UNRESOLVED,
};
use crate::domain::qualification::service::builtin_named;
use crate::domain::relations::collections::relations::implicit_symbol::constants as implicit;
use crate::domain::relations::collections::relations::implicit_symbol::item::{BUILTIN_ATTRIBUTES, MODULE_GLOBALS};
use ryo::languages::node_table::span_of;
use ryo::languages::{
    DefinesRow, FileWords, ImplicitSymbolRow, Integer, NameText, NodeId, Position, QualifiedNameRow, ReferenceRow, ResolvesRow,
    Rows, ScopeRow, Span, node_id,
};

/// The bytes of the node `at`.
pub(crate) fn span_at(read: ModelRead<'_>, at: Position) -> Span {
    let span = span_of(read.table, table_id(read.tree.segments, at));
    Span { start: span.start, end: span.end }
}

/// The node cell of the node `at` of the file being read.
pub(crate) fn node(read: ModelRead<'_>, at: Position) -> NodeId {
    node_id(read.file, table_id(read.tree.segments, at))
}

/// How many nodes the file's own tree has: the relations that state the file's own names leave the nodes of embedded
/// expressions out, which follow them.
pub(crate) fn own_count(read: ModelRead<'_>) -> Position {
    let all = u32::try_from(read.model.lexical.class.len()).unwrap_or(u32::MAX);
    Position(read.tree.segments.first().map_or(all, |segment| segment.first.0.min(all)))
}

/// The places of the nodes of the file's own tree, in id order.
fn own_positions(read: ModelRead<'_>) -> impl Iterator<Item = Position> + use<> {
    (0..own_count(read).0).map(Position)
}

/// The positions of the file's nodes, in id order.
fn positions(read: ModelRead<'_>) -> impl Iterator<Item = Position> + use<> {
    (0..u32::try_from(read.model.lexical.class.len()).unwrap_or(u32::MAX)).map(Position)
}

/// The row of the scope `at`: the scope and the scope it is evaluated in (the module is in itself); none when `at` opens no
/// namespace.
pub(crate) fn scope_row(read: ModelRead<'_>, at: Position) -> Option<ScopeRow> {
    let lexical = &read.model.lexical;
    let _: ScopeForm = scope_form(entry(&lexical.class, at))?;
    let parent = if at == MODULE { MODULE } else { scope_at(lexical, at) };
    Some(ScopeRow { parent: node(read, parent), child: node(read, at), file: read.file, span: span_at(read, at) })
}

/// Writes one row per scope.
pub(crate) fn scope_rows<V: ModelView>(file: &mut V, _words: &mut FileWords, rows: &mut dyn Rows<ScopeRow>) {
    let read = file.semantics(ModelPart::Lexical);
    for at in own_positions(read) {
        if let Some(row) = scope_row(read, at) {
            rows.write(row);
        }
    }
}

/// The word that says what a definition is, more finely than its kind.
const fn refinement(word: DefinitionWord) -> NameText<'static> {
    NameText(match word {
        DefinitionWord::Function => refinements::FUNCTION,
        DefinitionWord::AsyncFunction => refinements::ASYNC_FUNCTION,
        DefinitionWord::Method => refinements::METHOD,
        DefinitionWord::AsyncMethod => refinements::ASYNC_METHOD,
        DefinitionWord::Class => refinements::CLASS,
        DefinitionWord::Exception => refinements::EXCEPTION,
        DefinitionWord::Pattern => refinements::PATTERN,
        DefinitionWord::FromImport => refinements::FROM_IMPORT,
        DefinitionWord::Import => refinements::IMPORT,
        DefinitionWord::Variable => refinements::VARIABLE,
        DefinitionWord::TypeAlias => refinements::TYPE_ALIAS,
        DefinitionWord::PositionalOnly => refinements::POSITIONAL_ONLY,
        DefinitionWord::PositionalOrKeyword => refinements::POSITIONAL_OR_KEYWORD,
        DefinitionWord::KeywordOnly => refinements::KEYWORD_ONLY,
        DefinitionWord::VarPositional => refinements::VAR_POSITIONAL,
        DefinitionWord::VarKeyword => refinements::VAR_KEYWORD,
        DefinitionWord::TypeVar => refinements::TYPE_VAR,
        DefinitionWord::TypeVarTuple => refinements::TYPE_VAR_TUPLE,
        DefinitionWord::ParamSpec => refinements::PARAM_SPEC,
    })
}

/// The row of one definition.
fn defines_row(read: ModelRead<'_>, words: &mut FileWords, known: &mut FactWords, definition: &NameDefinition) -> DefinesRow {
    DefinesRow {
        id: node(read, definition.at),
        kind: definition.kind,
        refinement: fixed_word(words, &mut known.fixed, refinement(definition.word)),
        name: name_word(&read.model.names, words, &mut known.names, definition.name),
        owner: node(read, definition.owner),
        file: read.file,
        span: definition.named,
    }
}

/// Writes one row per definition, in source order.
pub(crate) fn defines_rows<V: ModelView>(file: &mut V, words: &mut FileWords, rows: &mut dyn Rows<DefinesRow>) {
    let read = file.semantics(ModelPart::Lexical);
    let mut known = read.model.words.borrow_mut();
    for definition in &read.model.definitions.all {
        rows.write(defines_row(read, words, &mut known, definition));
    }
}

/// The name expression the name or attribute expression `at` starts from: itself, or the innermost object of a chain of
/// attributes; none when the chain starts from another kind of expression.
fn base_of(model: &SemanticModel, at: Position) -> Option<Position> {
    let mut base = at;
    loop {
        let class = entry(&model.lexical.class, base);
        if class == NodeClass::Name {
            return Some(base);
        }
        if class != NodeClass::Attribute {
            return None;
        }
        base = Position(base.0.saturating_add(1));
    }
}

/// Appends to `path` the attributes read from the base of the chain up to `at`, each after a dot.
fn append_attributes(model: &SemanticModel, at: Position, path: &mut QualifiedPath) {
    if entry(&model.lexical.class, at) != NodeClass::Attribute {
        return;
    }
    append_attributes(model, Position(at.0.saturating_add(1)), path);
    path.0.push_str(DOT);
    if let Some(text) = text_of(&model.names, name_of(&model.lexical, at)) {
        path.0.push_str(text.0);
    }
}

/// The row of the name or attribute expression `at`, when it is read or deleted and is not the object of an attribute that is
/// read: the dotted name it writes, left in `path`.
pub(crate) fn reference_row(
    read: ModelRead<'_>,
    words: &mut FileWords,
    path: &mut QualifiedPath,
    at: Position,
) -> Option<ReferenceRow> {
    let lexical = &read.model.lexical;
    let class = entry(&lexical.class, at);
    let usage = entry(&lexical.usage, at);
    let used = (class == NodeClass::Name && usage != NameUse::Store)
        || (class == NodeClass::Attribute && usage == NameUse::Load);
    if !used {
        return None;
    }
    let parent = parent_of(read.tree, at);
    let object_of_read = entry(&lexical.class, parent) == NodeClass::Attribute
        && parent.0.saturating_add(1) == at.0
        && entry(&lexical.usage, parent) == NameUse::Load;
    if object_of_read {
        return None;
    }
    let base = base_of(read.model, at)?;
    path.0.clear();
    if let Some(text) = text_of(&read.model.names, name_of(lexical, base)) {
        path.0.push_str(text.0);
    }
    append_attributes(read.model, at, path);
    Some(ReferenceRow {
        occurrence: node(read, at),
        owner: node(read, scope_at(lexical, at)),
        target: word(words, NameText(&path.0)),
        file: read.file,
        span: span_at(read, at),
    })
}

/// Writes one row per name or attribute expression that is read or deleted and is not the object of an attribute that is read.
pub(crate) fn reference_rows<V: ModelView>(file: &mut V, words: &mut FileWords, rows: &mut dyn Rows<ReferenceRow>) {
    let read = file.semantics(ModelPart::Lexical);
    let mut path = QualifiedPath::default();
    for at in own_positions(read) {
        if let Some(row) = reference_row(read, words, &mut path, at) {
            rows.write(row);
        }
    }
}

/// The row of the name expression `at`, when it is read and `resolved` gives its name a definition of the file: the model's
/// resolution part for every name, or the static rule asked of this name alone.
pub(crate) fn resolves_row(
    read: ModelRead<'_>,
    words: &mut FileWords,
    known: &mut FactWords,
    at: Position,
    resolved: impl FnOnce(NameCode) -> Option<Position>,
) -> Option<ResolvesRow> {
    let lexical = &read.model.lexical;
    if entry(&lexical.class, at) != NodeClass::Name || entry(&lexical.usage, at) != NameUse::Load {
        return None;
    }
    let definition = resolved(name_of(lexical, at))?;
    Some(ResolvesRow {
        occurrence: node(read, at),
        owner: node(read, scope_at(lexical, at)),
        name: name_word(&read.model.names, words, &mut known.names, name_of(lexical, at)),
        definition: node(read, definition),
        file: read.file,
        span: span_at(read, at),
    })
}

/// Writes one row per name expression that is read and resolves to a definition of the file.
pub(crate) fn resolves_rows<V: ModelView>(file: &mut V, words: &mut FileWords, rows: &mut dyn Rows<ResolvesRow>) {
    let read = file.semantics(ModelPart::Resolution);
    let mut known = read.model.words.borrow_mut();
    for at in own_positions(read) {
        let resolved = |_name| resolved_at(&read.model.resolution, at);
        if let Some(row) = resolves_row(read, words, &mut known, at, resolved) {
            rows.write(row);
        }
    }
}

/// Writes into `path` the name an import brings, as its statement writes it.
fn import_path(names: &NameTable, origin: &ImportOrigin, path: &mut QualifiedPath) {
    if let Some(level) = origin.level {
        for _ in 0..level.0 {
            path.0.push_str(DOT);
        }
        if let Some(module) = text_of(names, origin.module) {
            path.0.push_str(module.0);
            path.0.push_str(DOT);
        }
    }
    let written = if origin.level.is_some() || origin.rename == ImportRename::Renamed { origin.name } else { origin.head };
    if let Some(text) = text_of(names, written) {
        path.0.push_str(text.0);
    }
}

/// Where the name or attribute expression `at` comes from, with its path left in `path` when it has one. It reads the model's
/// lexical part and its static resolution, and no node table.
pub(crate) fn qualify(model: &SemanticModel, at: Position, path: &mut QualifiedPath) -> NameProvenance {
    let Some(base) = base_of(model, at) else {
        return NameProvenance::Unresolved;
    };
    let name = name_of(&model.lexical, base);
    let binding = resolved_at(&model.resolution, base);
    if proof_of(&model.definitions, &model.lexical, &model.names, base, name, binding) == OriginProof::Unproven {
        return NameProvenance::Ambiguous;
    }
    path.0.clear();
    let provenance = if let Some(found) = binding {
        let Ok(place) = model.lexical.imports.binary_search_by_key(&found, |origin| origin.at) else {
            return NameProvenance::Local;
        };
        let Some(origin) = model.lexical.imports.get(place) else {
            return NameProvenance::Local;
        };
        import_path(&model.names, origin, path);
        NameProvenance::Import
    } else if let Some(since) = builtin_of(&model.names, name) {
        path.0.push_str(BUILTIN_MODULE);
        path.0.push_str(DOT);
        if let Some(text) = text_of(&model.names, name) {
            path.0.push_str(text.0);
        }
        NameProvenance::Builtin(since)
    } else {
        return NameProvenance::Unresolved;
    };
    append_attributes(model, at, path);
    provenance
}

/// The row of the name or attribute expression `at`, when it is read: where it comes from, and its qualified name when it has
/// one.
fn qualified_name_row(
    read: ModelRead<'_>,
    words: &mut FileWords,
    labels: ProvenanceWords,
    path: &mut QualifiedPath,
    at: Position,
) -> Option<QualifiedNameRow> {
    let lexical = &read.model.lexical;
    if !matches!(entry(&lexical.class, at), NodeClass::Name | NodeClass::Attribute)
        || entry(&lexical.usage, at) != NameUse::Load
    {
        return None;
    }
    let provenance = qualify(read.model, at, path);
    let (label, since) = match provenance {
        NameProvenance::Import => (labels.import, NOT_APPLICABLE),
        NameProvenance::Builtin(since) => (labels.builtin, since),
        NameProvenance::Local => (labels.local, NOT_APPLICABLE),
        NameProvenance::Unresolved => (labels.unresolved, NOT_APPLICABLE),
        NameProvenance::Ambiguous => (labels.ambiguous, NOT_APPLICABLE),
    };
    let has_path = matches!(provenance, NameProvenance::Import | NameProvenance::Builtin(_));
    Some(QualifiedNameRow {
        id: node(read, at),
        path: if has_path { word(words, NameText(&path.0)) } else { unknown(Unresolved::QualifiedOrigin) },
        provenance: label,
        since_major: since.major,
        since_minor: since.minor,
        file: read.file,
        span: span_at(read, at),
    })
}

/// Writes one row per name or attribute expression that is read.
pub(crate) fn qualified_name_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<QualifiedNameRow>,
) {
    let read = file.semantics(ModelPart::Resolution);
    if read.syntax == ryo::languages::SyntaxCompleteness::Incomplete {
        rows.partial(ryo::languages::ReportText(crate::domain::qualification::constants::INCOMPLETE), ryo::languages::Missing::Any);
        return;
    }
    let mut path = QualifiedPath::default();
    let labels = {
        let held = &mut read.model.words.borrow_mut().fixed;
        ProvenanceWords {
            import: fixed_word(words, held, NameText(refinements::IMPORT)),
            builtin: fixed_word(words, held, NameText(BUILTIN)),
            local: fixed_word(words, held, NameText(LOCAL)),
            unresolved: fixed_word(words, held, NameText(UNRESOLVED)),
            ambiguous: fixed_word(words, held, NameText(AMBIGUOUS)),
        }
    };
    for at in positions(read) {
        if let Some(row) = qualified_name_row(read, words, labels, &mut path, at) {
            rows.write(row);
        }
    }
}

/// Writes the catalog of names the language supplies to every module: the same rows for every file, owned by its module.
pub(crate) fn implicit_symbol_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<ImplicitSymbolRow>,
) {
    let read = file.semantics(ModelPart::Nothing);
    let scope = node(read, MODULE);
    let held = &mut read.model.words.borrow_mut().fixed;
    let builtin = fixed_word(words, held, NameText(implicit::BUILTIN));
    for name in BUILTIN_NAMES {
        if let Some(since) = builtin_named(*name) {
            rows.write(ImplicitSymbolRow {
                name: fixed_word(words, held, *name),
                kind: builtin,
                scope,
                since_major: since.major,
                since_minor: since.minor,
                file: read.file,
            });
        }
    }
    let module_global = fixed_word(words, held, NameText(implicit::MODULE_GLOBAL));
    for (catalog, kind) in [(BUILTIN_ATTRIBUTES, builtin), (MODULE_GLOBALS, module_global)] {
        for entry in catalog {
            rows.write(ImplicitSymbolRow {
                name: fixed_word(words, held, entry.name),
                kind,
                scope,
                since_major: entry.major,
                since_minor: entry.minor,
                file: read.file,
            });
        }
    }
    rows.write(ImplicitSymbolRow {
        name: fixed_word(words, held, NameText(implicit::WINDOWSERROR)),
        kind: fixed_word(words, held, NameText(implicit::PLATFORM_BUILTIN)),
        scope,
        since_major: Integer(3),
        since_minor: Integer(0),
        file: read.file,
    });
}
