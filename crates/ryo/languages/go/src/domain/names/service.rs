//! The `names` service: free functions named by verb over the domain's types, with no IO and no ports. A name a row holds is made from
//! the tokens of the file, in the file's names; what cannot be resolved statically is the unknown value of its reason.

use ryo::languages::{self, Position, Symbol, Tree};

use super::constants::{BLANK, DOT};
use super::types::{Adjacency, Blankness, Callee, Dottedness, Placement, Unresolved};
use crate::domain::constraints::service::{argument, kind_of, written as constraint_written};
use crate::domain::constraints::types::SourceBytes as ConstraintText;
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::{child, kind, nodes, parent, token, written};
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::constants::{BLOCK_COMMENT_CLOSE, BLOCK_COMMENT_OPEN, LINE_COMMENT};
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

/// The node an expression is a selection or an instantiation of: its operand.
fn operand(tree: &Tree<'_>, at: Position) -> Option<Position> {
    nodes(tree, at).next()
}

/// The identifier a selection selects: its last node.
fn selected(tree: &Tree<'_>, at: Position) -> Option<Position> {
    nodes(tree, at)
        .last()
        .filter(|each| kind(tree, *each) == Kind::Node(Node::Ident))
}

/// Whether the expression at `at` writes a dotted name.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only a name, a selection and an instantiation with several types write a dotted name; every other kind of the grammar writes none"
)]
pub(crate) fn dottedness(tree: &Tree<'_>, at: Position) -> Dottedness {
    let mut at = at;
    loop {
        match kind(tree, at) {
            Kind::Node(Node::Ident) => return Dottedness::Dotted,
            Kind::Node(Node::SelectorExpr) if selected(tree, at).is_some() => {
                match operand(tree, at) {
                    Some(value) => at = value,
                    None => return Dottedness::Other,
                }
            }
            // A type list is an instantiation of a generic function or type; one index may be an index of a slice or a map.
            Kind::Node(Node::IndexListExpr) => match operand(tree, at) {
                Some(value) => at = value,
                None => return Dottedness::Other,
            },
            _ => return Dottedness::Other,
        }
    }
}

/// Adds the dotted name the expression at `at` writes to the name being made; the expression is known to write one.
fn push_dotted(tree: &Tree<'_>, symbols: &mut languages::Symbols, at: Position) {
    match kind(tree, at) {
        Kind::Node(Node::Ident) => {
            if let Some(text) = languages::first_token_text(tree.elements, at) {
                languages::push_name(symbols, tree.source, text);
            }
        }
        Kind::Node(Node::SelectorExpr) => {
            if let Some(value) = operand(tree, at) {
                push_dotted(tree, symbols, value);
            }
            languages::push_text(symbols, languages::NameText(DOT));
            if let Some(name) = selected(tree, at) {
                push_dotted(tree, symbols, name);
            }
        }
        Kind::Node(Node::IndexListExpr) => {
            if let Some(value) = operand(tree, at) {
                push_dotted(tree, symbols, value);
            }
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => {}
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
    if kind(tree, at) == Kind::Node(Node::Ident) {
        return languages::first_token_text(tree.elements, at);
    }
    let from = languages::begin_name(symbols);
    push_dotted(tree, symbols, at);
    Some(languages::end_name(symbols, tree.source, from))
}

/// The token of the string literal at `at` that holds its text: nothing for a literal of another kind.
#[must_use]
pub(crate) fn string_token(tree: &Tree<'_>, at: Position) -> Option<Position> {
    child(tree, at, Kind::Token(Token::String))
}

/// The value of the string literal at `at`, as a name, or the unknown value of a string that is no text.
pub(crate) fn string_or_unknown(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Symbol {
    string_token(tree, at)
        .and_then(|token| languages::decoded_at(tree.elements, token))
        .map_or(unknown(Unresolved::NotText), |decoded| {
            languages::intern_derived(symbols, tree.source, decoded)
        })
}

/// Whether the string at `at` is an import specifier: the path of an `import` spec.
#[must_use]
pub(crate) fn specifier(tree: &Tree<'_>, at: Position) -> languages::Flag {
    if kind(tree, parent(tree, at)) == Kind::Node(Node::ImportSpec) {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}

/// The name an import binds: its own name when it has one, `.` and `_` read as what they mean.
pub(crate) fn import_binding(tree: &Tree<'_>, at: Position) -> Symbol {
    let mut written = nodes(tree, at);
    let (first, second) = (written.next(), written.next());
    match (first, second) {
        (Some(alias), Some(_)) => {
            if child(tree, alias, Kind::Token(Token::Period)).is_some() {
                unknown(Unresolved::DotImport)
            } else {
                languages::first_token_text(tree.elements, alias)
                    .unwrap_or(unknown(Unresolved::DeclaredByPackage))
            }
        }
        _ => unknown(Unresolved::DeclaredByPackage),
    }
}

/// The package an import names: the path it writes, or the unknown value of a path that is no text.
pub(crate) fn import_target(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Option<Symbol> {
    let path = nodes(tree, at).last()?;
    Some(string_or_unknown(tree, symbols, path))
}

/// The name an identifier writes, as a name of the file: the text of its token.
#[must_use]
pub(crate) fn name_of(tree: &Tree<'_>, ident: Position) -> Option<Symbol> {
    (kind(tree, ident) == Kind::Node(Node::Ident))
        .then(|| languages::first_token_text(tree.elements, ident))
        .flatten()
}

/// Whether the identifier at `ident` is the blank identifier `_`.
#[must_use]
pub(crate) fn blankness(tree: &Tree<'_>, ident: Position) -> Blankness {
    if written(tree, ident).0 == BLANK {
        Blankness::Blank
    } else {
        Blankness::Named
    }
}

/// The name an embedded field gets from its type at `at`: the last part of the type's base, `Type` for `*pkg.Type[int]`.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a type that is not a selection is named by its own identifier, whatever its kind"
)]
pub(crate) fn embedded_name(tree: &Tree<'_>, at: Position) -> Option<Symbol> {
    let at = base(tree, at)?;
    match kind(tree, at) {
        Kind::Node(Node::SelectorExpr) => {
            let each = nodes(tree, at).last()?;
            name_of(tree, each)
        }
        _ => name_of(tree, at),
    }
}

/// The base of a type: a pointer, a parenthesized type and the type arguments of a generic one are stripped.
fn base(tree: &Tree<'_>, at: Position) -> Option<Position> {
    let mut at = at;
    while matches!(
        kind(tree, at),
        Kind::Node(Node::StarExpr | Node::ParenExpr | Node::IndexExpr | Node::IndexListExpr)
    ) {
        at = nodes(tree, at).next()?;
    }
    Some(at)
}

/// The type the expression at `at` names, as the dotted name of its base: a pointer, a parenthesized type and the type arguments of a
/// generic one are stripped, so `*pkg.Type[int]` is `pkg.Type`.
pub(crate) fn base_of(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Option<Symbol> {
    let at = base(tree, at)?;
    dotted(tree, symbols, at)
}

/// What a call whose callee is the expression at `callee` calls.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a callee is a type written out or a dotted name; every other expression is called dynamically"
)]
pub(crate) fn callee_of(tree: &Tree<'_>, callee: Position) -> Callee {
    match kind(tree, callee) {
        Kind::Node(
            Node::ArrayType
            | Node::MapType
            | Node::ChanType
            | Node::FuncType
            | Node::StructType
            | Node::InterfaceType,
        ) => Callee::Conversion,
        _ if dottedness(tree, callee) == Dottedness::Dotted => Callee::Named,
        _ => Callee::Dynamic,
    }
}

/// The text of the comment at `at` without its delimiters, as a name.
pub(crate) fn comment_text(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Symbol {
    let text = written(tree, at).0;
    let inner = text.strip_prefix(LINE_COMMENT).unwrap_or_else(|| {
        text.strip_prefix(BLOCK_COMMENT_OPEN).map_or(text, |rest| {
            rest.strip_suffix(BLOCK_COMMENT_CLOSE).unwrap_or(rest)
        })
    });
    let from = languages::begin_name(symbols);
    languages::push_text(symbols, languages::NameText(inner));
    languages::end_name(symbols, tree.source, from)
}

/// Whether the comment at `at` is documentation, as `go/doc` reads it: a comment that starts its line, alone or in a group of such
/// comments with no blank line between them, directly above a declaration (a function, a method, a `const`, `var`, `type` or `import`
/// declaration or one of its specs, a field, or the package clause), with no blank line between the group and the declaration.
#[must_use]
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "a comment documents only the kinds that are declarations; every other kind ends the group"
)]
pub(crate) fn documentation(tree: &Tree<'_>, at: Position) -> languages::Flag {
    if placement(tree, at) == Placement::AfterCode {
        return languages::Flag::Lowered;
    }
    let mut last = at;
    for next in languages::children(tree.elements, parent(tree, at))
        .skip_while(|each| *each != at)
        .skip(1)
    {
        if kind(tree, next) == Kind::Extra(Extra::Whitespace) {
            continue;
        }
        if adjacency(tree, last, next) == Adjacency::Apart {
            return languages::Flag::Lowered;
        }
        match kind(tree, next) {
            Kind::Token(Token::Comment) => last = next,
            Kind::Node(
                Node::FuncDecl
                | Node::GenDecl
                | Node::TypeSpec
                | Node::ValueSpec
                | Node::ImportSpec
                | Node::Field,
            )
            | Kind::Token(Token::Package) => return languages::Flag::Raised,
            _ => return languages::Flag::Lowered,
        }
    }
    languages::Flag::Lowered
}

/// Whether nothing but blank space is written before the element at `at` on its line.
fn placement(tree: &Tree<'_>, at: Position) -> Placement {
    let start = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX);
    let before = tree.source.0.get(..start).unwrap_or_default();
    let line = before
        .iter()
        .rposition(|byte| *byte == b'\n')
        .map_or(before, |newline| {
            before.get(newline + 1..).unwrap_or_default()
        });
    if line.iter().all(u8::is_ascii_whitespace) {
        Placement::LineStart
    } else {
        Placement::AfterCode
    }
}

/// Whether `next` starts on the line after the one `last` ends on, with only blank space between them.
fn adjacency(tree: &Tree<'_>, last: Position, next: Position) -> Adjacency {
    let from = usize::try_from(languages::end_at(tree.elements, last).0).unwrap_or(usize::MAX);
    let to = usize::try_from(languages::start_at(tree.elements, next).0).unwrap_or(usize::MAX);
    let gap = tree.source.0.get(from..to).unwrap_or_default();
    if gap.iter().all(u8::is_ascii_whitespace) && gap.split(|byte| *byte == b'\n').count() == 2 {
        Adjacency::NextLine
    } else {
        Adjacency::Apart
    }
}

/// The name of the build constraint line at `at`, as the `directive` relation gives it: `go:build` or `+build`.
pub(crate) fn directive_kind(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Symbol {
    let line = ConstraintText(written(tree, at).0.as_bytes());
    let kind = std::str::from_utf8(kind_of(constraint_written(line)).0).unwrap_or_default();
    languages::intern_derived(symbols, tree.source, languages::NameText(kind))
}

/// The expression of the build constraint line at `at`, as `go/build` reads it: the line without its word and the white space around it.
pub(crate) fn directive_text(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: Position,
) -> Symbol {
    let line = ConstraintText(written(tree, at).0.as_bytes());
    let expression = std::str::from_utf8(argument(line).0).unwrap_or_default();
    languages::intern_derived(symbols, tree.source, languages::NameText(expression))
}

/// A name that no file writes, as a name of the run: the path of a module a manifest names.
pub(crate) fn named(symbols: &mut languages::Symbols, text: languages::NameText<'_>) -> Symbol {
    languages::intern_derived(symbols, languages::ByteSlice(&[]), text)
}

/// The arguments the call at `call` writes, in order; the callee is no argument.
pub(crate) fn arguments_of<'a>(
    tree: &'a Tree<'_>,
    call: Position,
) -> impl Iterator<Item = Position> + 'a {
    nodes(tree, call).skip(1)
}

/// The name the argument at `argument` hands on: the one name it writes, behind an address-of, a pointer dereference or parentheses;
/// nothing (the empty name) when it is anything else.
#[must_use]
pub(crate) fn passed(tree: &Tree<'_>, argument: Position) -> Symbol {
    let mut current = argument;
    loop {
        match kind(tree, current) {
            Kind::Node(Node::Ident) => {
                return name_of(tree, current).unwrap_or(languages::NO_SYMBOL);
            }
            Kind::Node(Node::ParenExpr | Node::StarExpr) => {}
            Kind::Node(Node::UnaryExpr) if token(tree, current, Token::And).is_some() => {}
            Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => return languages::NO_SYMBOL,
        }
        match nodes(tree, current).next() {
            Some(inner) => current = inner,
            None => return languages::NO_SYMBOL,
        }
    }
}
