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
/// The file's embedded expressions are classified once; each occurrence is then a carrier lookup
/// and a bisection of the hidden roots.
pub(crate) fn visibility(
    tree: &Tree<'_>,
    model: &Declarations,
    at: Position,
    symbols: &mut Symbols,
) -> Visibility {
    let Some(embedding) = languages::embedded_source_at(tree.elements, at) else {
        return Visibility::Visible;
    };
    if model.semantic.borrow().state == QualificationState::Fresh {
        let mut hidden = std::mem::take(&mut model.semantic.borrow_mut().hidden);
        hidden.clear();
        let provenance = prepare(tree, model);
        for expression in languages::embedded_expressions(tree.elements) {
            // A holder is appended before the expressions nested in it, so it is already decided.
            if hidden_at(tree, &hidden, expression.host) == Visibility::Hidden
                || type_position(tree, model, &provenance, expression.host, symbols)
                    != TypeContext::Type
            {
                hidden.push(expression.root);
            }
        }
        let mut semantic = model.semantic.borrow_mut();
        semantic.hidden = hidden;
        semantic.state = QualificationState::Ready;
    }
    if model
        .semantic
        .borrow()
        .hidden
        .binary_search(&embedding.root)
        .is_ok()
    {
        Visibility::Hidden
    } else {
        Visibility::Visible
    }
}

/// Hidden when the embedded expression holding `at` has its root among the ascending `hidden`.
fn hidden_at(tree: &Tree<'_>, hidden: &[Position], at: Position) -> Visibility {
    if languages::embedded_source_at(tree.elements, at)
        .is_some_and(|holder| hidden.binary_search(&holder.root).is_ok())
    {
        Visibility::Hidden
    } else {
        Visibility::Visible
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
    if super::generic_subscription::operands(path) == TypeContext::Type {
        return TypingForm::Generic;
    }
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
pub(crate) fn type_position(
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
    for ancestor in ancestors(tree, host) {
        // Slice bounds are runtime values even when their receiver is a known generic class.
        if kind(tree, ancestor) == Kind::Node(Node::Slice) {
            return TypeContext::Value;
        }
        if kind(tree, ancestor) == Kind::Node(Node::Subscript)
            && let Some(base) = nth(tree, ancestor, Category::Expression, CategoryIndex(0))
        {
            let name = typing_name(tree, model, provenance, base, symbols);
            if name == TypingForm::Literal {
                return TypeContext::Value;
            }
            if name == TypingForm::Annotated && !established {
                if annotated_operand(tree, ancestor, host) == TypeContext::Value {
                    return TypeContext::Value;
                }
                allowed = true;
                established = true;
            }
            if name == TypingForm::Generic
                && !established
                && nth(tree, ancestor, Category::Expression, CategoryIndex(1)) == Some(below)
            {
                allowed = true;
                established = true;
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
    }
    if allowed {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}

/// The ancestors that can change a host's type context, nearest first, through its statement:
/// no slice, subscription, call or annotated assignment holds a statement.
pub(crate) fn ancestors<'a>(
    tree: &'a Tree<'_>,
    host: Position,
) -> impl Iterator<Item = Position> + 'a {
    let mut below = host;
    std::iter::from_fn(move || {
        let ancestor = parent(tree, below);
        if ancestor == below || category(tree, below) == Category::Statement {
            return None;
        }
        below = ancestor;
        Some(ancestor)
    })
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
    operand_place(nodes(tree, arguments), argument, form, |at| {
        category(tree, at)
    })
}

/// Whether a non-keyword `argument` among the call's `operands` is a type operand of `form`.
/// Reads up to the second operand, or to the first expression for `TypeVar`; never the whole list.
pub(crate) fn operand_place(
    mut operands: impl Iterator<Item = Position>,
    argument: Position,
    form: TypingForm,
    mut category: impl FnMut(Position) -> Category,
) -> TypeContext {
    let is_type = match form {
        TypingForm::Cast => operands.next() == Some(argument),
        TypingForm::NewType => operands.nth(1) == Some(argument),
        TypingForm::TypeVar => {
            category(argument) == Category::Expression
                && operands.find(|at| category(*at) == Category::Expression) != Some(argument)
        }
        TypingForm::Literal
        | TypingForm::Annotated
        | TypingForm::TypeAlias
        | TypingForm::Other
        | TypingForm::Generic => false,
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
    // The host lies under the subscription, so the walk ends there at the latest.
    let mut owner = host;
    while parent(tree, owner) != slice && owner != subscription {
        owner = parent(tree, owner);
    }
    if owner == first {
        TypeContext::Type
    } else {
        TypeContext::Value
    }
}

/// Resolved native type carriers not already expanded by the native writer.
/// This classification intentionally leaves the semantic-root cache Fresh until publication.
pub(crate) fn completion_hosts(
    tree: &Tree<'_>,
    model: &Declarations,
    symbols: &mut Symbols,
) -> Vec<Position> {
    let provenance = prepare(tree, model);
    let expanded = languages::embedded_expressions(tree.elements)
        .map(|source| source.host)
        .collect::<std::collections::HashSet<_>>();
    languages::positions(tree.elements)
        .filter(|&host| {
            kind(tree, host) == Kind::Node(Node::Constant)
                && crate::domain::syntax::service::literal(tree, host)
                    == crate::domain::syntax::types::Literal::String
                && !expanded.contains(&host)
                && type_position(tree, model, &provenance, host, symbols) == TypeContext::Type
        })
        .collect()
}
