//! The `definitions` service of the model: free functions named by verb, with no IO and no ports.

use super::constants::NO_DEFINITION;
use super::types::{DefinitionCode, DefinitionModel, DefinitionRange, DefinitionWord, NameDefinition, PlacedParameter};
use crate::domain::model::lexical::constants::{NO_NODE, WILDCARD};
use crate::domain::syntax::constants::DOT;
use crate::domain::model::declarations::service::binding_scope;
use crate::domain::model::lexical::service::{entry, id_of, name_of, node_at, scope_at};
use crate::domain::model::lexical::types::{LexicalModel, NameUse, NamespaceKey, NodeClass, TypedTree};
use crate::domain::model::names::constants::UNKNOWN_NAME;
use crate::domain::model::names::service::find;
use crate::domain::model::names::types::{NameCode, NameTable};
use ruff_python_ast::{AnyNodeRef, Identifier, ParameterWithDefault};
use ryo::languages::node_table::{parent_of, span_of};
use ryo::languages::{Count, DeclarationKind, NameText, NodeTable, Offset, Position, Span};

/// The definition of the node `at`, when it defines a name.
#[must_use]
pub(crate) fn definition_of(model: &DefinitionModel, at: Position) -> Option<&NameDefinition> {
    let code = model.of_node.get(usize::from(at)).copied()?;
    model.all.get(usize::from(code))
}

/// The bytes of the node `at`.
fn span_at(table: &NodeTable, at: Position) -> Span {
    let span = span_of(table, at);
    Span { start: span.start, end: span.end }
}

/// A definition named by the identifier `identifier`, owned by the namespace the node `at` binds in.
fn named(
    lexical: &LexicalModel,
    table: &NodeTable,
    at: Position,
    identifier: &Identifier,
    kind: DeclarationKind,
    word: DefinitionWord,
) -> NameDefinition {
    let place = id_of(identifier);
    let name = name_of(lexical, place);
    NameDefinition { at, name, owner: binding_scope(lexical, table, at, name), kind, word, named: span_at(table, place) }
}

/// How the parameter `at` takes its argument, and the callable it belongs to.
fn placed(lexical: &LexicalModel, tree: TypedTree<'_, '_>, at: Position) -> Option<PlacedParameter> {
    let holder = parent_of(tree.table, at);
    let element = if entry(&lexical.class, holder) == NodeClass::ParameterWithDefault { holder } else { at };
    let parameters = parent_of(tree.table, element);
    let Some(AnyNodeRef::Parameters(typed)) = node_at(tree, parameters) else {
        return None;
    };
    let is = |parameter: &ParameterWithDefault| id_of(parameter) == element;
    let word = if typed.posonlyargs.iter().any(is) {
        DefinitionWord::PositionalOnly
    } else if typed.args.iter().any(is) {
        DefinitionWord::PositionalOrKeyword
    } else if typed.kwonlyargs.iter().any(is) {
        DefinitionWord::KeywordOnly
    } else if typed.vararg.as_deref().is_some_and(|parameter| id_of(parameter) == element) {
        DefinitionWord::VarPositional
    } else if typed.kwarg.as_deref().is_some_and(|parameter| id_of(parameter) == element) {
        DefinitionWord::VarKeyword
    } else {
        return None;
    };
    Some(PlacedParameter { word, callable: parent_of(tree.table, parameters) })
}

/// The definition a type parameter makes: its owner is the definition the parameters belong to.
fn type_parameter(
    lexical: &LexicalModel,
    table: &NodeTable,
    at: Position,
    identifier: &Identifier,
    word: DefinitionWord,
) -> NameDefinition {
    let place = id_of(identifier);
    NameDefinition {
        at,
        name: name_of(lexical, place),
        owner: parent_of(table, parent_of(table, at)),
        kind: DeclarationKind::TypeParameter,
        word,
        named: span_at(table, place),
    }
}

/// The definition an imported name makes: `import a.b` binds `a`, an `as` binds its name, `*` binds names unknown here.
fn imported(
    lexical: &LexicalModel,
    names: &NameTable,
    tree: TypedTree<'_, '_>,
    at: Position,
    alias: &ruff_python_ast::Alias,
) -> Option<NameDefinition> {
    let from = entry(&lexical.class, parent_of(tree.table, at)) == NodeClass::ImportFrom;
    let word = if from { DefinitionWord::FromImport } else { DefinitionWord::Import };
    let written = alias.name.as_str();
    let start = Offset(alias.name.range.start().to_u32());
    let (name, span) = if written == WILDCARD.0 {
        (UNKNOWN_NAME, Span { start, end: Offset(alias.name.range.end().to_u32()) })
    } else if let Some(renamed) = &alias.asname {
        let place = id_of(renamed);
        (name_of(lexical, place), span_at(tree.table, place))
    } else {
        let head = written.split(DOT).next().unwrap_or_default();
        let length = u32::try_from(head.len()).unwrap_or(u32::MAX);
        (find(names, NameText(head))?, Span { start, end: Offset(start.0.saturating_add(length)) })
    };
    Some(NameDefinition {
        at,
        name,
        owner: binding_scope(lexical, tree.table, at, name),
        kind: DeclarationKind::Import,
        word,
        named: span,
    })
}

/// The definition the node `at` makes, when it makes one.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "eleven of the parser's node kinds define a name; every other defines none"
)]
fn definition(lexical: &LexicalModel, names: &NameTable, tree: TypedTree<'_, '_>, at: Position) -> Option<NameDefinition> {
    let table = tree.table;
    match node_at(tree, at)? {
        AnyNodeRef::StmtFunctionDef(function) => {
            let method = entry(&lexical.class, scope_at(lexical, at)) == NodeClass::ClassDef;
            let (kind, word) = match (method, function.is_async) {
                (true, true) => (DeclarationKind::Method, DefinitionWord::AsyncMethod),
                (true, false) => (DeclarationKind::Method, DefinitionWord::Method),
                (false, true) => (DeclarationKind::Function, DefinitionWord::AsyncFunction),
                (false, false) => (DeclarationKind::Function, DefinitionWord::Function),
            };
            Some(named(lexical, table, at, &function.name, kind, word))
        }
        AnyNodeRef::StmtClassDef(class) => {
            Some(named(lexical, table, at, &class.name, DeclarationKind::Class, DefinitionWord::Class))
        }
        AnyNodeRef::ExceptHandlerExceptHandler(handler) => Some(named(
            lexical,
            table,
            at,
            handler.name.as_ref()?,
            DeclarationKind::Variable,
            DefinitionWord::Exception,
        )),
        AnyNodeRef::PatternMatchAs(pattern) => Some(named(
            lexical,
            table,
            at,
            pattern.name.as_ref()?,
            DeclarationKind::Variable,
            DefinitionWord::Pattern,
        )),
        AnyNodeRef::PatternMatchStar(pattern) => Some(named(
            lexical,
            table,
            at,
            pattern.name.as_ref()?,
            DeclarationKind::Variable,
            DefinitionWord::Pattern,
        )),
        AnyNodeRef::PatternMatchMapping(pattern) => Some(named(
            lexical,
            table,
            at,
            pattern.rest.as_ref()?,
            DeclarationKind::Variable,
            DefinitionWord::Pattern,
        )),
        AnyNodeRef::Alias(alias) => imported(lexical, names, tree, at, alias),
        AnyNodeRef::ExprName(_) => {
            if entry(&lexical.usage, at) != NameUse::Store {
                return None;
            }
            let name = name_of(lexical, at);
            let parent = parent_of(table, at);
            let alias = entry(&lexical.class, parent) == NodeClass::TypeAlias && parent.0.saturating_add(1) == at.0;
            let (kind, word) = if alias {
                (DeclarationKind::TypeAlias, DefinitionWord::TypeAlias)
            } else {
                (DeclarationKind::Variable, DefinitionWord::Variable)
            };
            Some(NameDefinition {
                at,
                name,
                owner: binding_scope(lexical, table, at, name),
                kind,
                word,
                named: span_at(table, at),
            })
        }
        AnyNodeRef::Parameter(parameter) => {
            let PlacedParameter { word, callable } = placed(lexical, tree, at)?;
            let place = id_of(&parameter.name);
            Some(NameDefinition {
                at,
                name: name_of(lexical, place),
                owner: callable,
                kind: DeclarationKind::Parameter,
                word,
                named: span_at(table, place),
            })
        }
        AnyNodeRef::TypeParamTypeVar(typed) => {
            Some(type_parameter(lexical, table, at, &typed.name, DefinitionWord::TypeVar))
        }
        AnyNodeRef::TypeParamTypeVarTuple(typed) => {
            Some(type_parameter(lexical, table, at, &typed.name, DefinitionWord::TypeVarTuple))
        }
        AnyNodeRef::TypeParamParamSpec(typed) => {
            Some(type_parameter(lexical, table, at, &typed.name, DefinitionWord::ParamSpec))
        }
        _ => None,
    }
}

/// The definitions of the name `name` in the namespace of `owner`, in source order.
#[must_use]
pub(crate) fn definitions_of(model: &DefinitionModel, owner: Position, name: NameCode) -> &[DefinitionCode] {
    model
        .index
        .get(&NamespaceKey { scope: owner, name })
        .and_then(|range| model.sorted.get(range.start.0..range.end.0))
        .unwrap_or_default()
}

/// Where the definition `code` is.
pub(crate) fn place_of(model: &DefinitionModel, code: DefinitionCode) -> Position {
    model.all.get(usize::from(code)).map_or(NO_NODE, |definition| definition.at)
}

/// Builds the definitions part of the model for one file, from its lexical part.
pub(crate) fn build(model: &mut DefinitionModel, lexical: &LexicalModel, names: &NameTable, tree: TypedTree<'_, '_>) {
    model.all.clear();
    model.of_node.clear();
    model.sorted.clear();
    model.index.clear();
    let count = lexical.class.len();
    model.of_node.resize(count, NO_DEFINITION);
    for at in &lexical.binders {
        if let Some(found) = definition(lexical, names, tree, *at) {
            let code = DefinitionCode(u32::try_from(model.all.len()).unwrap_or(u32::MAX));
            if let Some(slot) = model.of_node.get_mut(usize::from(*at)) {
                *slot = code;
            }
            model.all.push(found);
        }
    }
    model
        .sorted
        .extend((0..u32::try_from(model.all.len()).unwrap_or(u32::MAX)).map(DefinitionCode));
    let all = &model.all;
    model.sorted.sort_unstable_by_key(|code| {
        all.get(usize::from(*code))
            .map(|definition| (definition.owner, definition.name, definition.at))
    });
    let mut start = 0;
    while let Some(first) = model.sorted.get(start).and_then(|code| model.all.get(usize::from(*code))) {
        let key = NamespaceKey { scope: first.owner, name: first.name };
        let mut end = start.saturating_add(1);
        while model
            .sorted
            .get(end)
            .and_then(|code| model.all.get(usize::from(*code)))
            .is_some_and(|next| next.owner == key.scope && next.name == key.name)
        {
            end = end.saturating_add(1);
        }
        let _: Option<DefinitionRange> = model
            .index
            .insert(key, DefinitionRange { start: Count(start), end: Count(end) });
        start = end;
    }
}
