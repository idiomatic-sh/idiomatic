//! Select parsed annotation expressions using lexical typing context, never lint rules.
use super::constants::{
    ANNOTATED, BOUND, CAST, DEFAULT, LITERAL, NEW_TYPE, NEW_TYPE_KEYWORD, TYPE_ALIAS, TYPE_KEYWORD,
    TYPE_VAR, TYPING, TYPING_EXTENSIONS,
};
use super::types::{TypeContext, TypingForm, Visibility};
use crate::domain::syntax::constants::{ANNOTATION_CONTEXT, ANNOTATION_SEED};
use crate::domain::{
    extras::types::Extra,
    model::types::{Declarations, QualificationState},
    nodes::types::{Category, Node},
    qualification::service::{prepare, qualify},
    scopes::{service::nth, types::CategoryIndex},
    syntax::{
        service::{category, child, identifier_text, kind, nodes, parent},
        types::Kind,
    },
};
use ryo::languages::{self, Position, Symbols, Tree};

/// Every stable identity available to the explicit semantic projection.
pub(crate) fn positions(tree: &Tree<'_>) -> impl Iterator<Item = Position> {
    (0..languages::end_of(tree.elements).0).map(Position)
}

/// Whether an occurrence belongs to native syntax or an enabled embedded expression.
pub(crate) fn visibility(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    symbols: &mut Symbols,
) -> Visibility {
    let Some(embedding) = languages::embedded_expressions(tree.elements)
        .find(|embedding| embedding.root <= at && at < embedding.end)
    else {
        return Visibility::Visible;
    };
    if model.semantic.borrow().state == QualificationState::Fresh {
        let provenance = prepare(tree, model);
        let mut semantic = model.semantic.borrow_mut();
        semantic.roots.clear();
        for expression in languages::embedded_expressions(tree.elements) {
            let parent_enabled = languages::embedded_expressions(tree.elements)
                .find(|outer| outer.root <= expression.host && expression.host < outer.end)
                .is_none_or(|outer| semantic.roots.contains(&outer.root));
            if parent_enabled
                && type_position(tree, model, &provenance, expression.host, symbols)
                    == TypeContext::Type
            {
                semantic.roots.push(expression.root);
            }
        }
        semantic.state = QualificationState::Ready;
    }
    if model.semantic.borrow().roots.contains(&embedding.root) {
        Visibility::Visible
    } else {
        Visibility::Hidden
    }
}

/// Resolve a typing form using the same lexical provenance as ordinary expressions.
fn typing_name(
    tree: &Tree<'_>,
    model: &Declarations,
    provenance: &crate::domain::model::types::QualificationIndex,
    expression: Position,
    symbols: &mut Symbols,
) -> TypingForm {
    let answer = qualify(tree, model, provenance, expression, symbols);
    let Some(path) = languages::text_of(symbols, tree.source, answer.path) else {
        return TypingForm::Other;
    };
    match path
        .0
        .strip_prefix(TYPING)
        .or_else(|| path.0.strip_prefix(TYPING_EXTENSIONS))
    {
        Some(LITERAL) => TypingForm::Literal,
        Some(ANNOTATED) => TypingForm::Annotated,
        Some(CAST) => TypingForm::Cast,
        Some(NEW_TYPE) => TypingForm::NewType,
        Some(TYPE_VAR) => TypingForm::TypeVar,
        Some(TYPE_ALIAS) => TypingForm::TypeAlias,
        Some(_) | None => TypingForm::Other,
    }
}

/// `Literal` arguments and `Annotated` metadata are values. Ordinary callable
/// arguments also leave type context; `cast`'s first argument remains a type.
fn type_position(
    tree: &Tree<'_>,
    model: &Declarations,
    provenance: &crate::domain::model::types::QualificationIndex,
    host: Position,
    symbols: &mut Symbols,
) -> TypeContext {
    let mut allowed = languages::property_at(tree.elements, host, ANNOTATION_CONTEXT)
        .is_some_and(|context| context.0 == ANNOTATION_SEED);
    let mut established = false;
    let mut below = host;
    let mut ancestor = parent(tree, below);
    while ancestor != below {
        if kind(tree, ancestor) == Kind::Node(Node::Subscript)
            && let Some(base) = nth(tree, ancestor, Category::Expression, CategoryIndex(0))
        {
            let name = typing_name(tree, model, provenance, base, symbols);
            if name == TypingForm::Literal {
                return TypeContext::Value;
            }
            if name == TypingForm::Annotated
                && !established
                && annotated_operand(tree, ancestor, host) == TypeContext::Value
            {
                return TypeContext::Value;
            }
        }
        if kind(tree, ancestor) == Kind::Extra(Extra::CallArguments) && !established {
            let call = parent(tree, ancestor);
            let Some(function) = nth(tree, call, Category::Expression, CategoryIndex(0)) else {
                return TypeContext::Value;
            };
            let name = typing_name(tree, model, provenance, function, symbols);
            if type_argument(tree, ancestor, below, name) == TypeContext::Value {
                return TypeContext::Value;
            }
            allowed = true;
            established = true;
        }
        if kind(tree, ancestor) == Kind::Node(Node::AnnAssign) && !allowed {
            let Some(annotation) = nth(tree, ancestor, Category::Expression, CategoryIndex(1))
            else {
                return TypeContext::Value;
            };
            if annotation != below
                && typing_name(tree, model, provenance, annotation, symbols)
                    == TypingForm::TypeAlias
            {
                allowed = true;
                established = true;
            }
        }
        below = ancestor;
        ancestor = parent(tree, ancestor);
    }
    if allowed {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}

/// Classify positional and keyword type operands after resolving the callable.
fn type_argument(
    tree: &Tree<'_>,
    arguments: Position,
    argument: Position,
    form: TypingForm,
) -> TypeContext {
    if kind(tree, argument) == Kind::Node(Node::Keyword) {
        let name = child(tree, argument, Kind::Extra(Extra::Identifier))
            .map(|identifier| identifier_text(tree, identifier).0);
        return if matches!(
            (form, name),
            (TypingForm::Cast, Some(TYPE_KEYWORD))
                | (TypingForm::NewType, Some(NEW_TYPE_KEYWORD))
                | (TypingForm::TypeVar, Some(BOUND | DEFAULT))
        ) {
            TypeContext::Type
        } else {
            TypeContext::Value
        };
    }
    let source_position = nodes(tree, arguments).position(|at| at == argument);
    let positional = nodes(tree, arguments)
        .filter(|at| category(tree, *at) == Category::Expression)
        .position(|at| at == argument);
    let is_type = match form {
        TypingForm::Cast => source_position == Some(0),
        TypingForm::NewType => source_position == Some(1),
        TypingForm::TypeVar => positional.is_some_and(|position| position > 0),
        TypingForm::Literal | TypingForm::Annotated | TypingForm::TypeAlias | TypingForm::Other => {
            false
        }
    };
    if is_type {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}

/// Only the first element of an Annotated subscription is a type operand.
fn annotated_operand(tree: &Tree<'_>, subscription: Position, host: Position) -> TypeContext {
    let Some(slice) = nth(tree, subscription, Category::Expression, CategoryIndex(1)) else {
        return TypeContext::Value;
    };
    if kind(tree, slice) != Kind::Node(Node::Tuple) {
        return TypeContext::Type;
    }
    let Some(first) = nth(tree, slice, Category::Expression, CategoryIndex(0)) else {
        return TypeContext::Value;
    };
    let mut owner = host;
    while parent(tree, owner) != slice && parent(tree, owner) != owner {
        owner = parent(tree, owner);
    }
    if owner == first {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}
