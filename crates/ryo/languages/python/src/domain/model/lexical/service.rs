//! The `lexical` service of the model: free functions named by verb, with no IO and no ports. One pass in id order says what each
//! node is, the scope it is evaluated in (a definition's decorators, defaults, annotations and bases are evaluated around it, a
//! comprehension's first iterable around the comprehension), the names of the file, and its declarations and hazards.

use super::constants::{MODULE, WILDCARD};
use super::types::{
    AnnotationMark, ImportLevel, ImportOrigin, ImportRename, LexicalModel, NameHazard, NameUse, NodeClass, ScopeForm,
    SyntaxForm, TypedTree,
};
use crate::domain::model::names::constants::NO_NAME;
use crate::domain::model::names::service::intern;
use crate::domain::model::names::types::{NameCode, NameTable};
use crate::domain::syntax::constants::DOT;
use ruff_python_ast::{Alias, AnyNodeRef, ExprContext, HasNodeIndex as _};
use ruff_text_size::Ranged as _;
use ryo::languages::{Count, NameText, Offset, Position, Span};

/// The id of a node reached in place (a typed child), through the node index the walk wrote.
#[must_use]
pub(crate) fn id_of<'a>(node: impl Into<AnyNodeRef<'a>>) -> Position {
    Position(node.into().node_index().load().as_u32().unwrap_or(u32::MAX))
}

/// How many nodes the tree has.
#[must_use]
pub(crate) const fn node_count(tree: TypedTree<'_, '_>) -> Count {
    Count(tree.nodes.len())
}

/// The parent of the node `at`; the root's is itself.
#[must_use]
pub(crate) fn parent_of(tree: TypedTree<'_, '_>, at: Position) -> Position {
    tree.parents.get(usize::from(at)).copied().unwrap_or(at)
}

/// Where the subtree of the node `at` ends: the place after its last descendant.
#[must_use]
pub(crate) fn subtree_end_of(tree: TypedTree<'_, '_>, at: Position) -> Position {
    tree.ends.get(usize::from(at)).copied().unwrap_or(Position(at.0.saturating_add(1)))
}

/// The bytes the node `at` is written in, as the parser read them.
#[must_use]
pub(crate) fn span_of(tree: TypedTree<'_, '_>, at: Position) -> Span {
    let (start, end) =
        node_at(tree, at).map_or((Offset(0), Offset(0)), |node| (Offset(node.start().to_u32()), Offset(node.end().to_u32())));
    Span { start, end }
}

/// The parser's node of the id `at`.
#[must_use]
pub(crate) fn node_at<'a>(tree: TypedTree<'_, 'a>, at: Position) -> Option<AnyNodeRef<'a>> {
    tree.nodes.get(usize::from(at)).copied()
}

/// The entry of a per-node column for `at`; the column's default past its end.
#[must_use]
pub(crate) fn entry<T: Copy + Default>(column: &[T], at: Position) -> T {
    column.get(usize::from(at)).copied().unwrap_or_default()
}

/// Writes the entry of a per-node column for `at`.
pub(crate) fn put<T>(column: &mut [T], at: Position, value: T) {
    if let Some(slot) = column.get_mut(usize::from(at)) {
        *slot = value;
    }
}

/// The scope the node `at` is evaluated in.
#[must_use]
pub(crate) fn scope_at(model: &LexicalModel, at: Position) -> Position {
    model.scope_of.get(usize::from(at)).copied().unwrap_or(MODULE)
}

/// The name the node `at` carries; the code of no name when it carries none.
#[must_use]
pub(crate) fn name_of(model: &LexicalModel, at: Position) -> NameCode {
    model.name.get(usize::from(at)).copied().unwrap_or(NO_NAME)
}

/// What the parser's node is to the name rules.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "the name rules tell thirty-two of the parser's node kinds apart; every other is a statement, an expression or neither"
)]
fn class_of(node: AnyNodeRef<'_>) -> NodeClass {
    match node {
        AnyNodeRef::ModModule(_) | AnyNodeRef::ModExpression(_) => NodeClass::Module,
        AnyNodeRef::StmtFunctionDef(_) => NodeClass::FunctionDef,
        AnyNodeRef::StmtClassDef(_) => NodeClass::ClassDef,
        AnyNodeRef::ExprLambda(_) => NodeClass::Lambda,
        AnyNodeRef::ExprListComp(_) => NodeClass::ListComp,
        AnyNodeRef::ExprSetComp(_) => NodeClass::SetComp,
        AnyNodeRef::ExprDictComp(_) => NodeClass::DictComp,
        AnyNodeRef::ExprGenerator(_) => NodeClass::Generator,
        AnyNodeRef::Comprehension(_) => NodeClass::Comprehension,
        AnyNodeRef::ExprName(_) => NodeClass::Name,
        AnyNodeRef::ExprAttribute(_) => NodeClass::Attribute,
        AnyNodeRef::Identifier(_) => NodeClass::Identifier,
        AnyNodeRef::Alias(_) => NodeClass::Alias,
        AnyNodeRef::Parameter(_) => NodeClass::Parameter,
        AnyNodeRef::Parameters(_) => NodeClass::Parameters,
        AnyNodeRef::ParameterWithDefault(_) => NodeClass::ParameterWithDefault,
        AnyNodeRef::ExceptHandlerExceptHandler(_) => NodeClass::ExceptHandler,
        AnyNodeRef::PatternMatchAs(_) => NodeClass::MatchAs,
        AnyNodeRef::PatternMatchStar(_) => NodeClass::MatchStar,
        AnyNodeRef::PatternMatchMapping(_) => NodeClass::MatchMapping,
        AnyNodeRef::TypeParamTypeVar(_) => NodeClass::TypeVar,
        AnyNodeRef::TypeParamTypeVarTuple(_) => NodeClass::TypeVarTuple,
        AnyNodeRef::TypeParamParamSpec(_) => NodeClass::ParamSpec,
        AnyNodeRef::TypeParams(_) => NodeClass::TypeParams,
        AnyNodeRef::StmtGlobal(_) => NodeClass::Global,
        AnyNodeRef::StmtNonlocal(_) => NodeClass::Nonlocal,
        AnyNodeRef::ExprNamed(_) => NodeClass::NamedExpr,
        AnyNodeRef::StmtTypeAlias(_) => NodeClass::TypeAlias,
        AnyNodeRef::StmtAnnAssign(_) => NodeClass::AnnAssign,
        AnyNodeRef::StmtAssign(_) => NodeClass::Assign,
        AnyNodeRef::StmtImportFrom(_) => NodeClass::ImportFrom,
        AnyNodeRef::StmtImport(_) => NodeClass::Import,
        other if other.is_statement() => NodeClass::Statement,
        other if other.is_expression() => NodeClass::Expression,
        _ => NodeClass::Other,
    }
}

/// How a node of the class `class` opens a namespace, when it opens one.
#[must_use]
pub(crate) const fn scope_form(class: NodeClass) -> Option<ScopeForm> {
    if matches!(class, NodeClass::ListComp | NodeClass::SetComp | NodeClass::DictComp) {
        return Some(ScopeForm::Comprehension);
    }
    if matches!(class, NodeClass::Module) {
        Some(ScopeForm::Module)
    } else if matches!(class, NodeClass::FunctionDef) {
        Some(ScopeForm::Function)
    } else if matches!(class, NodeClass::ClassDef) {
        Some(ScopeForm::Class)
    } else if matches!(class, NodeClass::Lambda) {
        Some(ScopeForm::Lambda)
    } else if matches!(class, NodeClass::Generator) {
        Some(ScopeForm::Generator)
    } else {
        None
    }
}

/// Whether a node of the class `class` is a statement, an expression or neither.
#[must_use]
pub(crate) const fn syntax_form(class: NodeClass) -> SyntaxForm {
    if matches!(
        class,
        NodeClass::Statement
            | NodeClass::FunctionDef
            | NodeClass::ClassDef
            | NodeClass::Global
            | NodeClass::Nonlocal
            | NodeClass::TypeAlias
            | NodeClass::AnnAssign
            | NodeClass::Assign
            | NodeClass::ImportFrom
            | NodeClass::Import
    ) {
        SyntaxForm::Statement
    } else if matches!(
        class,
        NodeClass::Expression
            | NodeClass::Name
            | NodeClass::Attribute
            | NodeClass::Lambda
            | NodeClass::ListComp
            | NodeClass::SetComp
            | NodeClass::DictComp
            | NodeClass::Generator
            | NodeClass::NamedExpr
    ) {
        SyntaxForm::Expression
    } else {
        SyntaxForm::Neither
    }
}

/// The scope a child of the scope node `parent` is evaluated in: the scope itself for what runs inside it (a definition's body
/// and type parameters, a lambda's expressions), the scope around it for the rest.
fn evaluated(model: &LexicalModel, parent: Position, parent_class: NodeClass, child_class: NodeClass) -> Position {
    let inside = if matches!(parent_class, NodeClass::FunctionDef | NodeClass::ClassDef) {
        syntax_form(child_class) == SyntaxForm::Statement || child_class == NodeClass::TypeParams
    } else if parent_class == NodeClass::Lambda {
        syntax_form(child_class) == SyntaxForm::Expression
    } else {
        true
    };
    if inside { parent } else { scope_at(model, parent) }
}

/// The comprehension whose first clause is `clause`, when `child` is that clause's iterable: it is evaluated around the
/// comprehension.
fn first_iterable(model: &LexicalModel, tree: TypedTree<'_, '_>, clause: Position, child: Position) -> Option<Position> {
    let Some(AnyNodeRef::Comprehension(typed)) = node_at(tree, clause) else {
        return None;
    };
    if id_of(&typed.iter) != child {
        return None;
    }
    let owner = parent_of(tree, clause);
    let end = subtree_end_of(tree, owner);
    let mut first = Position(owner.0.saturating_add(1));
    while first < end {
        if entry(&model.class, first) == NodeClass::Comprehension {
            return (first == clause).then_some(owner);
        }
        first = subtree_end_of(tree, first);
    }
    None
}

/// Forgets the file, keeping the buffers.
fn clear(model: &mut LexicalModel) {
    model.class.clear();
    model.scope_of.clear();
    model.name.clear();
    model.usage.clear();
    model.annotation.clear();
    model.declared.clear();
    model.binders.clear();
    model.deleted.clear();
    model.hazards.clear();
    model.imports.clear();
}

/// Records where the imported name of `alias` comes from, and interns the names it is read by.
fn import_origin(
    model: &mut LexicalModel,
    names: &mut NameTable,
    tree: TypedTree<'_, '_>,
    at: Position,
    alias: &Alias,
) {
    let written = alias.name.as_str();
    let head = written.split(DOT).next().unwrap_or_default();
    let (level, module) = if let Some(AnyNodeRef::StmtImportFrom(from)) = node_at(tree, parent_of(tree, at)) {
        (
            Some(ImportLevel(from.level)),
            from.module
                .as_ref()
                .map_or(NO_NAME, |module| intern(names, NameText(module.as_str()))),
        )
    } else {
        (None, NO_NAME)
    };
    model.imports.push(ImportOrigin {
        at,
        level,
        module,
        name: intern(names, NameText(written)),
        head: intern(names, NameText(head)),
        rename: if alias.asname.is_some() { ImportRename::Renamed } else { ImportRename::Kept },
    });
}

/// Reads the node `at` into the model: its name, what it does with it, the annotations it holds, and whether it may define a name.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "seventeen of the parser's node kinds carry a name, an annotation or a definition; every other carries none"
)]
fn read(model: &mut LexicalModel, names: &mut NameTable, tree: TypedTree<'_, '_>, at: Position, scope: Position) {
    let Some(node) = node_at(tree, at) else {
        return;
    };
    match node {
        AnyNodeRef::ExprName(name) => {
            put(&mut model.name, at, intern(names, NameText(name.id.as_str())));
            match name.ctx {
                ExprContext::Load | ExprContext::Invalid => {}
                ExprContext::Store => {
                    put(&mut model.usage, at, NameUse::Store);
                    model.binders.push(at);
                }
                ExprContext::Del => {
                    put(&mut model.usage, at, NameUse::Delete);
                    model.deleted.push(at);
                }
            }
        }
        AnyNodeRef::ExprAttribute(attribute) => {
            put(&mut model.name, at, intern(names, NameText(attribute.attr.as_str())));
            match attribute.ctx {
                ExprContext::Load | ExprContext::Invalid => {}
                ExprContext::Store => put(&mut model.usage, at, NameUse::Store),
                ExprContext::Del => put(&mut model.usage, at, NameUse::Delete),
            }
        }
        AnyNodeRef::Identifier(identifier) => {
            put(&mut model.name, at, intern(names, NameText(identifier.id.as_str())));
        }
        AnyNodeRef::StmtFunctionDef(function) => {
            if let Some(returns) = &function.returns {
                put(&mut model.annotation, id_of(&**returns), AnnotationMark::Annotation);
            }
            model.binders.push(at);
        }
        AnyNodeRef::Parameter(parameter) => {
            if let Some(annotation) = &parameter.annotation {
                put(&mut model.annotation, id_of(&**annotation), AnnotationMark::Annotation);
            }
            model.binders.push(at);
        }
        AnyNodeRef::StmtAnnAssign(assignment) => {
            let mark = if matches!(entry(&model.class, scope), NodeClass::FunctionDef | NodeClass::Lambda) {
                AnnotationMark::Unevaluated
            } else {
                AnnotationMark::Annotation
            };
            put(&mut model.annotation, id_of(&*assignment.annotation), mark);
        }
        AnyNodeRef::StmtClassDef(_)
        | AnyNodeRef::TypeParamTypeVar(_)
        | AnyNodeRef::TypeParamTypeVarTuple(_)
        | AnyNodeRef::TypeParamParamSpec(_) => model.binders.push(at),
        AnyNodeRef::ExceptHandlerExceptHandler(handler) if handler.name.is_some() => model.binders.push(at),
        AnyNodeRef::PatternMatchAs(pattern) if pattern.name.is_some() => model.binders.push(at),
        AnyNodeRef::PatternMatchStar(pattern) if pattern.name.is_some() => model.binders.push(at),
        AnyNodeRef::PatternMatchMapping(pattern) if pattern.rest.is_some() => model.binders.push(at),
        AnyNodeRef::Alias(alias) => {
            if alias.name.as_str() == WILDCARD.0 {
                model.hazards.push(NameHazard { scope, at, name: NO_NAME });
            }
            import_origin(model, names, tree, at, alias);
            model.binders.push(at);
        }
        _ => {}
    }
}

/// Builds the lexical part of the model for one file.
pub(crate) fn build(model: &mut LexicalModel, names: &mut NameTable, tree: TypedTree<'_, '_>) {
    clear(model);
    model.nodes = node_count(tree);
    let count = model.nodes.0.min(tree.nodes.len());
    model.class.extend(tree.nodes.iter().take(count).map(|node| class_of(*node)));
    model.scope_of.resize(count, MODULE);
    model.name.resize(count, NO_NAME);
    model.usage.resize(count, NameUse::Load);
    model.annotation.resize(count, AnnotationMark::Plain);
    for number in 1..u32::try_from(count).unwrap_or(u32::MAX) {
        let at = Position(number);
        let parent = parent_of(tree, at);
        let parent_class = entry(&model.class, parent);
        let scope = if parent == MODULE {
            MODULE
        } else if let Some(owner) = (parent_class == NodeClass::Comprehension)
            .then(|| first_iterable(model, tree, parent, at))
            .flatten()
        {
            scope_at(model, owner)
        } else if scope_form(parent_class).is_some() {
            evaluated(model, parent, parent_class, entry(&model.class, at))
        } else {
            scope_at(model, parent)
        };
        put(&mut model.scope_of, at, scope);
        read(model, names, tree, at, scope);
    }
}
