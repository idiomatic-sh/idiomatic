//! The `lexical_facts` service: free functions named by verb that write the rows of six relations from the model, with no IO and
//! no ports. Each has the parameters of its relation's filler; the per-file view is whatever implements `ModelView`.
//!
//! `resolves` and `qualified-name` keep their present meaning: one static answer per name that is read (the last definition
//! before it in source order, else the first in a function-like scope; `definitions::service` states the rule). Whether they
//! become the reaching set with the selection in the rule pack is an open decision of the product's owner.
#![cfg_attr(
    not(test),
    expect(
        dead_code,
        reason = "the fillers and the model they read have no caller in this library until the walk's `Fillers` implementation, which delegates to them, joins the branch"
    )
)]

use super::types::{NameWords, QualifiedPath, NameProvenance};
use crate::domain::definitions::constants as refinements;
use crate::domain::model::definitions::types::DefinitionWord;
use crate::domain::model::resolution::service::{proof_of, resolved_at};
use crate::domain::model::resolution::types::OriginProof;
use crate::domain::model::lexical::constants::MODULE;
use crate::domain::syntax::constants::DOT;
use crate::domain::model::lexical::service::{entry, name_of, scope_at, scope_form};
use crate::domain::model::lexical::types::{ImportOrigin, ImportRename, NameUse, NodeClass};
use crate::domain::model::names::service::{builtin_of, text_of};
use crate::domain::model::names::types::{NameCode, NameTable};
use crate::domain::model::view::types::{ModelPart, ModelRead, ModelView};
use crate::domain::names::service::unknown;
use crate::domain::names::types::Unresolved;
use crate::domain::qualification::constants::{
    AMBIGUOUS, BUILTIN, BUILTIN_MODULE, BUILTIN_NAMES, LOCAL, NOT_APPLICABLE, UNRESOLVED,
};
use crate::domain::qualification::service::builtin_named;
use crate::domain::relations::collections::relations::implicit_symbol::constants as implicit;
use crate::domain::relations::collections::relations::implicit_symbol::item::{BUILTIN_ATTRIBUTES, MODULE_GLOBALS};
use ryo::languages::node_table::{parent_of, span_of};
use ryo::languages::{
    DefinesRow, FileWords, Fill, ImplicitSymbolRow, Integer, NameText, NodeId, NodeTable, Position, QualifiedNameRow,
    ReferenceRow, ResolvesRow, Rows, ScopeRow, Span, Symbol, facts, node_id,
};

/// The word of `text` among the file's words.
pub(crate) fn word(words: &mut FileWords, text: NameText<'_>) -> Symbol {
    Symbol(u32::try_from(facts::intern(words, text).0).unwrap_or(u32::MAX))
}

/// The word of the name `code`, written once per filler; the unknown word of a wildcard import for the names one binds.
pub(crate) fn name_word(names: &NameTable, words: &mut FileWords, known: &mut NameWords, code: NameCode) -> Symbol {
    if let Some(Some(found)) = known.0.get(usize::from(code)) {
        return *found;
    }
    let Some(text) = text_of(names, code) else {
        return unknown(Unresolved::WildcardImport);
    };
    let found = word(words, text);
    if known.0.len() < names.entries.len() {
        known.0.resize(names.entries.len(), None);
    }
    if let Some(slot) = known.0.get_mut(usize::from(code)) {
        *slot = Some(found);
    }
    found
}

/// The bytes of the node `at`.
pub(crate) fn span_at(table: &NodeTable, at: Position) -> Span {
    let span = span_of(table, at);
    Span { start: span.start, end: span.end }
}

/// The node cell of the node `at` of the file being read.
pub(crate) fn node(read: ModelRead<'_>, at: Position) -> NodeId {
    node_id(read.file, at)
}

/// The positions of the file's nodes, in id order.
fn positions(read: ModelRead<'_>) -> impl Iterator<Item = Position> + use<> {
    (0..u32::try_from(read.model.lexical.class.len()).unwrap_or(u32::MAX)).map(Position)
}

/// Writes one row per scope: the scope and the scope it is evaluated in (the module is in itself).
pub(crate) fn scope_rows<V: ModelView>(file: &mut V, _words: &mut FileWords, rows: &mut dyn Rows<ScopeRow>) -> Fill {
    let read = file.semantics(ModelPart::Lexical);
    let lexical = &read.model.lexical;
    for at in positions(read) {
        if scope_form(entry(&lexical.class, at)).is_none() {
            continue;
        }
        let parent = if at == MODULE { MODULE } else { scope_at(lexical, at) };
        rows.write(ScopeRow {
            parent: node(read, parent),
            child: node(read, at),
            file: read.file,
            span: span_at(read.table, at),
        });
    }
    Fill::Written
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

/// Writes one row per definition, in source order.
pub(crate) fn defines_rows<V: ModelView>(file: &mut V, words: &mut FileWords, rows: &mut dyn Rows<DefinesRow>) -> Fill {
    let read = file.semantics(ModelPart::Lexical);
    let mut known = NameWords::default();
    for definition in &read.model.definitions.all {
        rows.write(DefinesRow {
            id: node(read, definition.at),
            kind: definition.kind,
            refinement: word(words, refinement(definition.word)),
            name: name_word(&read.model.names, words, &mut known, definition.name),
            owner: node(read, definition.owner),
            file: read.file,
            span: definition.named,
        });
    }
    Fill::Written
}

/// The name expression the name or attribute expression `at` starts from: itself, or the innermost object of a chain of
/// attributes; none when the chain starts from another kind of expression.
fn base_of(read: ModelRead<'_>, at: Position) -> Option<Position> {
    let mut base = at;
    loop {
        let class = entry(&read.model.lexical.class, base);
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
fn append_attributes(read: ModelRead<'_>, at: Position, path: &mut QualifiedPath) {
    if entry(&read.model.lexical.class, at) != NodeClass::Attribute {
        return;
    }
    append_attributes(read, Position(at.0.saturating_add(1)), path);
    path.0.push_str(DOT);
    if let Some(text) = text_of(&read.model.names, name_of(&read.model.lexical, at)) {
        path.0.push_str(text.0);
    }
}

/// Writes one row per name or attribute expression that is read or deleted and is not the object of an attribute that is read:
/// the dotted name it writes.
pub(crate) fn reference_rows<V: ModelView>(file: &mut V, words: &mut FileWords, rows: &mut dyn Rows<ReferenceRow>) -> Fill {
    let read = file.semantics(ModelPart::Lexical);
    let lexical = &read.model.lexical;
    let mut path = QualifiedPath::default();
    for at in positions(read) {
        let class = entry(&lexical.class, at);
        let usage = entry(&lexical.usage, at);
        let used = (class == NodeClass::Name && usage != NameUse::Store)
            || (class == NodeClass::Attribute && usage == NameUse::Load);
        if !used {
            continue;
        }
        let parent = parent_of(read.table, at);
        let object_of_read = entry(&lexical.class, parent) == NodeClass::Attribute
            && parent.0.saturating_add(1) == at.0
            && entry(&lexical.usage, parent) == NameUse::Load;
        if object_of_read {
            continue;
        }
        let Some(base) = base_of(read, at) else {
            continue;
        };
        path.0.clear();
        if let Some(text) = text_of(&read.model.names, name_of(lexical, base)) {
            path.0.push_str(text.0);
        }
        append_attributes(read, at, &mut path);
        rows.write(ReferenceRow {
            occurrence: node(read, at),
            owner: node(read, scope_at(lexical, at)),
            target: word(words, NameText(&path.0)),
            file: read.file,
            span: span_at(read.table, at),
        });
    }
    Fill::Written
}

/// Writes one row per name expression that is read and resolves to a definition of the file.
pub(crate) fn resolves_rows<V: ModelView>(file: &mut V, words: &mut FileWords, rows: &mut dyn Rows<ResolvesRow>) -> Fill {
    let read = file.semantics(ModelPart::Lexical);
    let lexical = &read.model.lexical;
    let mut known = NameWords::default();
    for at in positions(read) {
        if entry(&lexical.class, at) != NodeClass::Name || entry(&lexical.usage, at) != NameUse::Load {
            continue;
        }
        let Some(definition) = resolved_at(&read.model.resolution, at) else {
            continue;
        };
        rows.write(ResolvesRow {
            occurrence: node(read, at),
            owner: node(read, scope_at(lexical, at)),
            name: name_word(&read.model.names, words, &mut known, name_of(lexical, at)),
            definition: node(read, definition),
            file: read.file,
            span: span_at(read.table, at),
        });
    }
    Fill::Written
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

/// Where the name or attribute expression `at` comes from, with its path left in `path` when it has one.
fn qualify(read: ModelRead<'_>, at: Position, path: &mut QualifiedPath) -> NameProvenance {
    let model = read.model;
    let Some(base) = base_of(read, at) else {
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
    append_attributes(read, at, path);
    provenance
}

/// Writes one row per name or attribute expression that is read: where it comes from, and its qualified name when it has one.
pub(crate) fn qualified_name_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<QualifiedNameRow>,
) -> Fill {
    let read = file.semantics(ModelPart::Lexical);
    let lexical = &read.model.lexical;
    let mut path = QualifiedPath::default();
    let labels = [NameText(refinements::IMPORT), NameText(BUILTIN), NameText(LOCAL), NameText(UNRESOLVED), NameText(AMBIGUOUS)]
        .map(|label| word(words, label));
    let [import, builtin, local, unresolved, ambiguous] = labels;
    for at in positions(read) {
        if !matches!(entry(&lexical.class, at), NodeClass::Name | NodeClass::Attribute)
            || entry(&lexical.usage, at) != NameUse::Load
        {
            continue;
        }
        let (label, since_major, since_minor, has_path) = match qualify(read, at, &mut path) {
            NameProvenance::Import => (import, NOT_APPLICABLE.major, NOT_APPLICABLE.minor, true),
            NameProvenance::Builtin(since) => (builtin, since.major, since.minor, true),
            NameProvenance::Local => (local, NOT_APPLICABLE.major, NOT_APPLICABLE.minor, false),
            NameProvenance::Unresolved => (unresolved, NOT_APPLICABLE.major, NOT_APPLICABLE.minor, false),
            NameProvenance::Ambiguous => (ambiguous, NOT_APPLICABLE.major, NOT_APPLICABLE.minor, false),
        };
        rows.write(QualifiedNameRow {
            id: node(read, at),
            path: if has_path { word(words, NameText(&path.0)) } else { unknown(Unresolved::QualifiedOrigin) },
            provenance: label,
            since_major,
            since_minor,
            file: read.file,
            span: span_at(read.table, at),
        });
    }
    Fill::Written
}

/// Writes the catalog of names the language supplies to every module: the same rows for every file, owned by its module.
pub(crate) fn implicit_symbol_rows<V: ModelView>(
    file: &mut V,
    words: &mut FileWords,
    rows: &mut dyn Rows<ImplicitSymbolRow>,
) -> Fill {
    let read = file.semantics(ModelPart::Nothing);
    let scope = node(read, MODULE);
    let builtin = word(words, NameText(implicit::BUILTIN));
    for name in BUILTIN_NAMES {
        if let Some(since) = builtin_named(*name) {
            rows.write(ImplicitSymbolRow {
                name: word(words, *name),
                kind: builtin,
                scope,
                since_major: since.major,
                since_minor: since.minor,
                file: read.file,
            });
        }
    }
    let module_global = word(words, NameText(implicit::MODULE_GLOBAL));
    for (catalog, kind) in [(BUILTIN_ATTRIBUTES, builtin), (MODULE_GLOBALS, module_global)] {
        for entry in catalog {
            rows.write(ImplicitSymbolRow {
                name: word(words, entry.name),
                kind,
                scope,
                since_major: entry.major,
                since_minor: entry.minor,
                file: read.file,
            });
        }
    }
    rows.write(ImplicitSymbolRow {
        name: word(words, NameText(implicit::WINDOWSERROR)),
        kind: word(words, NameText(implicit::PLATFORM_BUILTIN)),
        scope,
        since_major: Integer(3),
        since_minor: Integer(0),
        file: read.file,
    });
    Fill::Written
}
