//! The `names` service: free functions named by verb over the domain's types, with no IO and no ports. A name a row holds is made
//! from the tokens of the file, in the file's names; what cannot be resolved statically is the unknown value of its reason.

use ryo::languages::{self, Position, Symbol, Tree};

use super::constants::DOT;
use super::types::{Annotation, Assignment, Dottedness, Knownness, NameReading, Unresolved, Use};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::{Category, Node};
use crate::domain::scopes::service::{context, nth, place_among};
use crate::domain::scopes::types::{CategoryIndex, Context};
use crate::domain::syntax::service::{
    below, child, identifier_text, interpolation, kind, literal, name_of, name_token, nodes,
    parent, written,
};
use crate::domain::syntax::types::Kind;
use crate::domain::syntax::types::{Interpolation, Literal};
use crate::domain::tokens::types::Token;

/// The unknown value of `reason`: the same word in every file.
#[must_use]
#[expect(
    clippy::as_conversions,
    reason = "a reason is a field-less enum with a u8 representation: its number is the cast"
)]
pub(crate) const fn unknown(reason: Unresolved) -> Symbol {
    languages::unknown_symbol(languages::UnknownReason(reason as u8))
}

/// Whether the expression at `at` writes a dotted name.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a name and an attribute write a dotted name; every other kind writes none"
)]
pub(crate) fn dottedness(tree: &Tree<'_>, at: Position) -> Dottedness {
    let mut at = at;
    loop {
        match kind(tree, at) {
            Kind::Node(Node::Name) if name_token(tree, at).is_some() => {
                return Dottedness::Dotted;
            }
            Kind::Node(Node::Attribute) => {
                match nth(tree, at, Category::Expression, CategoryIndex(0)) {
                    Some(value) if child(tree, at, Kind::Extra(Extra::Identifier)).is_some() => {
                        at = value;
                    }
                    _ => return Dottedness::Other,
                }
            }
            _ => return Dottedness::Other,
        }
    }
}

/// Adds the dotted name the expression at `at` writes to the name being made; the expression is known to write one.
fn push_dotted(tree: &Tree<'_>, symbols: &mut languages::Symbols, at: Position) {
    if kind(tree, at) == Kind::Node(Node::Attribute) {
        if let Some(value) = nth(tree, at, Category::Expression, CategoryIndex(0)) {
            push_dotted(tree, symbols, value);
        }
        languages::push_text(symbols, languages::NameText(DOT));
    }
    let named = if kind(tree, at) == Kind::Node(Node::Attribute) {
        child(tree, at, Kind::Extra(Extra::Identifier))
            .and_then(|identifier| name_token(tree, identifier))
    } else {
        name_token(tree, at)
    };
    if let Some(token) = named {
        languages::push_text(
            symbols,
            languages::NameText(identifier_text(tree, parent(tree, token)).0),
        );
    }
}

/// The dotted name the expression at `at` writes; nothing for any other expression.
pub(crate) fn dotted(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Option<Symbol> {
    if dottedness(tree, at) == Dottedness::Other {
        return None;
    }
    if kind(tree, at) == Kind::Node(Node::Name) {
        return name_of(tree, at);
    }
    let from = languages::begin_name(symbols);
    push_dotted(tree, symbols, at);
    Some(languages::end_name(symbols, tree.source, from))
}

/// The dotted name the expression at `at` writes, or the unknown value of `reason`.
pub(crate) fn dotted_or(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
    reason: Unresolved,
) -> Symbol {
    dotted(tree, symbols, at).unwrap_or(unknown(reason))
}

/// What a decorator names: the dotted name it writes, or the one it calls.
pub(crate) fn decorator_target(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    decorator: Position,
) -> Symbol {
    let Some(expression) = nth(tree, decorator, Category::Expression, CategoryIndex(0)) else {
        return unknown(Unresolved::DynamicDecorator);
    };
    let named = if kind(tree, expression) == Kind::Node(Node::Call) {
        nth(tree, expression, Category::Expression, CategoryIndex(0)).unwrap_or(expression)
    } else {
        expression
    };
    dotted_or(tree, symbols, named, Unresolved::DynamicDecorator)
}

/// The arguments the decorator at `decorator` passes, as written between the parentheses of its call (`"/", methods=["GET"]` of
/// `@app.route("/", methods=["GET"])`), or nothing when it calls nothing.
pub(crate) fn decorator_arguments(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    decorator: Position,
) -> Symbol {
    let between = nth(tree, decorator, Category::Expression, CategoryIndex(0))
        .filter(|expression| kind(tree, *expression) == Kind::Node(Node::Call))
        .and_then(|call| {
            let callee = nth(tree, call, Category::Expression, CategoryIndex(0))?;
            let after = languages::end_at(tree.elements, callee);
            let until = languages::end_at(tree.elements, call);
            let held = || {
                languages::positions(tree.elements)
                    .skip(usize::try_from(call.0).unwrap_or(usize::MAX))
                    .take_while(move |each| languages::start_at(tree.elements, *each) < until)
            };
            let open = held().find(|each| {
                kind(tree, *each) == Kind::Token(Token::Lpar)
                    && languages::start_at(tree.elements, *each) >= after
            })?;
            let close = held()
                .filter(|each| kind(tree, *each) == Kind::Token(Token::Rpar))
                .last()?;
            Some((
                languages::end_at(tree.elements, open),
                languages::start_at(tree.elements, close),
            ))
        });
    let begun = languages::begin_name(symbols);
    if let Some((from, to)) = between {
        languages::push_span(symbols, tree.source, from, to);
    }
    languages::end_name(symbols, tree.source, begun)
}

/// What a base of a class names: the dotted name it writes, or the one it parameterizes (`Generic[T]` is `Generic`); a base indexed
/// by a number (`bases[0]`) picks from a sequence, and is dynamic.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a base that is not starred or subscripted is read as a dotted name, whatever its kind"
)]
pub(crate) fn base_target(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    base: Position,
) -> Symbol {
    match kind(tree, base) {
        Kind::Node(Node::Starred) => unknown(Unresolved::StarBase),
        Kind::Node(Node::Subscript)
            if nth(tree, base, Category::Expression, CategoryIndex(1))
                .is_some_and(|index| literal(tree, index) == Literal::Number) =>
        {
            unknown(Unresolved::DynamicBase)
        }
        Kind::Node(Node::Subscript) => nth(tree, base, Category::Expression, CategoryIndex(0))
            .map_or(unknown(Unresolved::DynamicBase), |value| {
                dotted_or(tree, symbols, value, Unresolved::DynamicBase)
            }),
        _ => dotted_or(tree, symbols, base, Unresolved::DynamicBase),
    }
}

/// Adds the parts of the dotted name an identifier writes, `a.b`, joined by dots.
pub(crate) fn push_identifier(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    identifier: Position,
) {
    if let Some(canonical) = languages::property_at(
        tree.elements,
        identifier,
        crate::domain::syntax::constants::NORMALIZED_IDENTIFIER,
    ) {
        languages::push_text(symbols, canonical);
        return;
    }
    let mut first = true;
    for token in
        below(tree, identifier).filter(|each| kind(tree, *each) == Kind::Token(Token::Name))
    {
        if !first {
            languages::push_text(symbols, languages::NameText(DOT));
        }
        languages::push_text(symbols, languages::NameText(written(tree, token).0));
        first = false;
    }
}

/// The value of the string, f-string or t-string at `at`, its parts joined as the library decoded them; nothing when it interpolates.
pub(crate) fn string_value(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Option<Symbol> {
    if interpolation(tree, at) == Interpolation::Interpolates {
        return None;
    }
    let from = languages::begin_name(symbols);
    for token in below(tree, at).filter(|each| {
        matches!(
            kind(tree, *each),
            Kind::Token(Token::String | Token::FstringMiddle | Token::TstringMiddle)
        )
    }) {
        match languages::decoded_at(tree.elements, token) {
            Some(text) => languages::push_text(symbols, text),
            None => languages::push_text(symbols, languages::NameText(written(tree, token).0)),
        }
    }
    Some(languages::end_name(symbols, tree.source, from))
}

/// The value of the string at `at`, or the unknown value of an interpolated string.
pub(crate) fn string_or_unknown(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Symbol {
    string_value(tree, symbols, at).unwrap_or(unknown(Unresolved::InterpolatedString))
}

/// Whether the expression at `at` is a string whose value is known: a string literal, or an f-string with no interpolation.
#[must_use]
pub(crate) fn knownness(tree: &Tree<'_>, at: Position) -> Knownness {
    if (literal(tree, at) == Literal::String || kind(tree, at) == Kind::Node(Node::JoinedStr))
        && interpolation(tree, at) == Interpolation::Static
    {
        Knownness::Known
    } else {
        Knownness::Other
    }
}

/// Whether the expression at `at` is the object of an attribute that is read: part of the longer dotted name the attribute writes.
fn reading(tree: &Tree<'_>, at: Position) -> NameReading {
    let above = parent(tree, at);
    if kind(tree, above) == Kind::Node(Node::Attribute)
        && place_among(tree, above, at, Category::Expression) == Some(CategoryIndex(0))
        && context(tree, above) == Context::Load
    {
        NameReading::ObjectOfRead
    } else {
        NameReading::Own
    }
}

/// Whether the name or attribute expression at `at` is a reference of its own: a name read or deleted, or a dotted attribute read,
/// that is not the object of a longer dotted name read.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a name and an attribute are references; every other kind is none"
)]
pub(crate) fn use_of(tree: &Tree<'_>, at: Position) -> Use {
    let used = match kind(tree, at) {
        Kind::Node(Node::Name) => context(tree, at) != Context::Store,
        Kind::Node(Node::Attribute) => {
            context(tree, at) == Context::Load && dottedness(tree, at) == Dottedness::Dotted
        }
        _ => false,
    } && reading(tree, at) == NameReading::Own;
    if used { Use::Reference } else { Use::Other }
}

/// The place of the decorator at `at` among the decorators of its definition, from zero.
#[must_use]
pub(crate) fn decorator_ordinal(tree: &Tree<'_>, at: Position) -> languages::Integer {
    let place = nodes(tree, parent(tree, at))
        .filter(|each| kind(tree, *each) == Kind::Extra(Extra::Decorator))
        .position(|each| each == at)
        .unwrap_or(0);
    languages::Integer(i64::try_from(place).unwrap_or(i64::MAX))
}

/// The annotation the element at `at` has, and what it annotates: a parameter's type, a function's result, an annotated target's
/// type.
#[must_use]
pub(crate) fn annotation_of(tree: &Tree<'_>, at: Position) -> Option<Annotation> {
    if kind(tree, at) == Kind::Node(Node::AnnAssign) {
        let target = nth(tree, at, Category::Expression, CategoryIndex(0))?;
        return Some(Annotation {
            annotation: nth(tree, at, Category::Expression, CategoryIndex(1))?,
            target,
        });
    }
    Some(Annotation {
        annotation: nth(tree, at, Category::Expression, CategoryIndex(0))?,
        target: at,
    })
}

/// The names the assignment at `at` binds to a string whose value is known, each with the string: `x = "a"`, `a = b = "a"`,
/// `x: str = "a"`, `(x := "a")`. A target that is no single name binds no string here.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only an assignment, an annotated assignment and an assignment expression bind a value; any other kind binds none"
)]
pub(crate) fn assignments<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Assignment> + 'a {
    let count = crate::domain::syntax::service::nodes_of(tree, at, Category::Expression).count();
    let (targets, value) = match kind(tree, at) {
        Kind::Node(Node::Assign) => (count.saturating_sub(1), count.saturating_sub(1)),
        Kind::Node(Node::AnnAssign) if count == 3 => (1, 2),
        Kind::Node(Node::NamedExpr) => (1, 1),
        _ => (0, 0),
    };
    let value = nth(tree, at, Category::Expression, CategoryIndex(value))
        .filter(|each| knownness(tree, *each) == Knownness::Known);
    crate::domain::syntax::service::nodes_of(tree, at, Category::Expression)
        .take(targets)
        .filter(move |each| kind(tree, *each) == Kind::Node(Node::Name))
        .filter_map(move |binding| value.map(|value| Assignment { binding, value }))
}

/// The arguments the call at `call` writes, in the order they are written: the positional ones, then the keywords, each keyword as the
/// value it gives; the callee is no argument.
pub(crate) fn arguments_of<'a>(
    tree: &'a Tree<'_>,
    call: Position,
) -> impl Iterator<Item = Position> + 'a {
    child(tree, call, Kind::Extra(Extra::CallArguments))
        .into_iter()
        .flat_map(move |list| nodes(tree, list))
        .filter_map(move |each| {
            if kind(tree, each) == Kind::Node(Node::Keyword) {
                nth(tree, each, Category::Expression, CategoryIndex(0))
            } else {
                Some(each)
            }
        })
}

/// The name the argument at `argument` hands on: the one name it writes, behind a star; nothing (the empty name) when it is anything
/// else.
#[must_use]
pub(crate) fn passed(tree: &Tree<'_>, argument: Position) -> Symbol {
    let mut current = argument;
    while kind(tree, current) == Kind::Node(Node::Starred) {
        match nth(tree, current, Category::Expression, CategoryIndex(0)) {
            Some(inner) => current = inner,
            None => return languages::NO_SYMBOL,
        }
    }
    if kind(tree, current) == Kind::Node(Node::Name) {
        name_of(tree, current).unwrap_or(languages::NO_SYMBOL)
    } else {
        languages::NO_SYMBOL
    }
}
