//! The `syntax` service: free functions named by verb over the domain's types, with no IO and no
//! ports. It maps a kind to the number the element table holds and back, says its name, and reads the kind of an element of a file's
//! syntax tree: a position is an element of the table; its kind is the language's own.

use ryo::languages::{self, Tree};

use super::constants::{
    ARROW, BLOCK_CLOSE, BLOCK_OPEN, COMPUTED_OPEN, DOC_OPEN, EMPTY_BLOCK, HASHBANG, LINE_COMMENT,
    MEMBER_SEPARATOR, PARAMETERS_CLOSE, PARAMETERS_OPEN, PRIVATE_PREFIX, PROPERTY_SEPARATOR,
    REQUIRE,
};
use super::types::{Declaredness, Kind, Leaf};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::JavaScriptNode;
use crate::domain::tokens::types::Token;

ryo::languages::kinds!(pub Kind { Node(JavaScriptNode), Token(Token), Extra(Extra) } unknown Kind::Extra(Extra::Error));

/// The kind of the element at `at`.
#[must_use]
pub(crate) fn kind(tree: &Tree<'_>, at: languages::Position) -> Kind {
    kind_of(languages::kind_at(tree.elements, at))
}

/// Whether the element at `at` is a token, a leaf, and not a node.
#[must_use]
pub(crate) fn is_token(tree: &Tree<'_>, at: languages::Position) -> Leaf {
    if languages::flags_at(tree.elements, at).0 & languages::TOKEN.0 == 0 {
        Leaf::Node
    } else {
        Leaf::Token
    }
}

/// The parent of the element at `at`; the root is its own.
#[must_use]
pub(crate) fn parent(tree: &Tree<'_>, at: languages::Position) -> languages::Position {
    languages::parent_at(tree.elements, at)
}

/// The name of the kind of the element at `at`, as the `kind` column of the tree holds it.
pub(crate) fn kind_symbol(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let element = kind(tree, at);
    languages::vocabulary_symbol(symbols, tree.source, kind_bits(element), kind_name(element))
}

/// The text of the token at `at`; no text for a node.
#[must_use]
pub(crate) fn token_text(tree: &Tree<'_>, at: languages::Position) -> languages::Symbol {
    if is_token(tree, at) == Leaf::Token {
        languages::text_at(tree.elements, at)
    } else {
        languages::NO_SYMBOL
    }
}

/// Direct `ESTree` node children, excluding lexical leaves.
pub(crate) fn nodes<'a>(
    tree: &'a Tree<'_>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    languages::children(tree.elements, at)
        .filter(|child| matches!(kind(tree, *child), Kind::Node(_)))
}
/// Source spelling of one element, in bytes.
pub(crate) fn spelling<'a>(tree: &'a Tree<'_>, at: languages::Position) -> languages::NameText<'a> {
    let start = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX);
    let end = usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX);
    languages::NameText(
        std::str::from_utf8(tree.source.0.get(start..end).unwrap_or_default()).unwrap_or_default(),
    )
}
/// Use decoded identifier names, retaining the private-name sigil.
pub(crate) fn named(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    if let Some(value) = languages::children(tree.elements, at)
        .find_map(|child| languages::decoded_at(tree.elements, child))
    {
        if kind(tree, at) == Kind::Node(JavaScriptNode::PrivateIdentifier) {
            let mark = languages::begin_name(symbols);
            languages::push_text(symbols, languages::NameText(PRIVATE_PREFIX));
            languages::push_text(symbols, value);
            return languages::end_name(symbols, tree.source, mark);
        }
        return languages::intern_derived(symbols, tree.source, value);
    }
    languages::intern_derived(symbols, tree.source, spelling(tree, at))
}
/// No statically knowable single name exists for this expression.
pub(crate) fn unknown() -> languages::Symbol {
    languages::unknown_symbol(languages::UnknownReason(0))
}
/// Decoded contents of a string-valued `ESTree` literal.
pub(crate) fn string(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> Option<languages::Symbol> {
    let token = languages::children(tree.elements, at)
        .find(|child| kind(tree, *child) == Kind::Token(Token::String))?;
    languages::decoded_at(tree.elements, token)
        .map(|value| languages::intern_derived(symbols, tree.source, value))
}
/// Comment contents, retaining its original whitespace and the row's original source span.
pub(crate) fn comment(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    let text = spelling(tree, at).0;
    let value = text
        .strip_prefix(LINE_COMMENT)
        .or_else(|| text.strip_prefix(HASHBANG))
        .or_else(|| {
            let body = text.strip_prefix(BLOCK_OPEN)?;
            body.strip_suffix(BLOCK_CLOSE)
        })
        .unwrap_or(text);
    languages::intern_derived(symbols, tree.source, languages::NameText(value))
}
/// Whether the comment at `at` is documentation, as `JSDoc` and TypeScript read it: a block comment that opens with `/**`, the empty
/// `/**/` aside.
#[must_use]
pub(crate) fn documentation(tree: &Tree<'_>, at: languages::Position) -> languages::Flag {
    let text = spelling(tree, at).0;
    if text.starts_with(DOC_OPEN) && text != EMPTY_BLOCK {
        languages::Flag::Raised
    } else {
        languages::Flag::Lowered
    }
}
/// Formal parameters in source order, excluding the declaration name and body.
pub(crate) fn parameters<'a>(
    tree: &'a Tree<'_>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    let arrow = languages::children(tree.elements, at)
        .find(|child| spelling(tree, *child).0 == ARROW)
        .map(|child| languages::start_at(tree.elements, child));
    let before_body = |child: &languages::Position| {
        arrow.is_none_or(|end| languages::end_at(tree.elements, *child) <= end)
    };
    let open = languages::children(tree.elements, at)
        .filter(before_body)
        .find(|child| spelling(tree, *child).0 == PARAMETERS_OPEN)
        .map(|child| languages::end_at(tree.elements, child));
    let close = languages::children(tree.elements, at)
        .filter(before_body)
        .find(|child| spelling(tree, *child).0 == PARAMETERS_CLOSE)
        .map(|child| languages::start_at(tree.elements, child));
    let first = nodes(tree, at).next();
    nodes(tree, at).filter(move |child| match (open, close) {
        (Some(open), Some(close)) => {
            languages::start_at(tree.elements, *child) >= open
                && languages::end_at(tree.elements, *child) <= close
        }
        _ => kind(tree, at) == Kind::Node(JavaScriptNode::ArrowFunctionExpression) && Some(*child) == first,
    })
}
/// An explicit function name; anonymous expressions have no inferred name.
pub(crate) fn function_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    match function_form(tree, at) {
        Declaredness::Method(key) => {
            return key.map_or_else(unknown, |key| named(tree, symbols, key));
        }
        Declaredness::Expression => return unknown(),
        Declaredness::Function => {}
    }
    let open = languages::children(tree.elements, at)
        .find(|child| spelling(tree, *child).0 == PARAMETERS_OPEN)
        .map(|child| languages::start_at(tree.elements, child));
    nodes(tree, at)
        .find(|child| {
            kind(tree, *child) == Kind::Node(JavaScriptNode::Identifier)
                && open.is_some_and(|open| languages::start_at(tree.elements, *child) < open)
        })
        .map_or_else(unknown, |name| named(tree, symbols, name))
}
/// Whether the function at `at` is declared: a function declaration, the body of a class method, or the body of an object's method,
/// getter or setter (a property with no `:` between its key and its value); every other function is an expression.
pub(crate) fn function_form(tree: &Tree<'_>, at: languages::Position) -> Declaredness {
    match kind(tree, at) {
        Kind::Node(JavaScriptNode::FunctionDeclaration) => Declaredness::Function,
        Kind::Node(JavaScriptNode::FunctionExpression) => {
            let holder = parent(tree, at);
            let has = |spelled: languages::NameText<'_>| {
                languages::children(tree.elements, holder)
                    .any(|child| spelling(tree, child).0 == spelled.0)
            };
            let method = match kind(tree, holder) {
                Kind::Node(JavaScriptNode::MethodDefinition) => true,
                Kind::Node(JavaScriptNode::Property) => !has(languages::NameText(PROPERTY_SEPARATOR)),
                Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => false,
            };
            if !method {
                return Declaredness::Expression;
            }
            let key = nodes(tree, holder).next().filter(|key| {
                !has(languages::NameText(COMPUTED_OPEN))
                    && matches!(
                        kind(tree, *key),
                        Kind::Node(JavaScriptNode::Identifier | JavaScriptNode::PrivateIdentifier)
                    )
            });
            Declaredness::Method(key)
        }
        Kind::Node(_) | Kind::Token(_) | Kind::Extra(_) => Declaredness::Expression,
    }
}
/// Single parameter binding, or unknown for a destructuring pattern.
pub(crate) fn parameter_name(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    if kind(tree, at) == Kind::Node(JavaScriptNode::Identifier) {
        return named(tree, symbols, at);
    }
    if matches!(
        kind(tree, at),
        Kind::Node(JavaScriptNode::AssignmentPattern | JavaScriptNode::RestElement)
    ) {
        return nodes(tree, at)
            .next()
            .map_or_else(unknown, |child| parameter_name(tree, symbols, child));
    }
    unknown()
}
/// Statically spelled call target, never a computed property or evaluated expression.
pub(crate) fn target(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    if matches!(
        kind(tree, at),
        Kind::Node(JavaScriptNode::Identifier | JavaScriptNode::PrivateIdentifier | JavaScriptNode::ThisExpression | JavaScriptNode::Super)
    ) {
        return named(tree, symbols, at);
    }
    if kind(tree, at) == Kind::Node(JavaScriptNode::MemberExpression)
        && !languages::children(tree.elements, at)
            .any(|child| spelling(tree, child).0 == COMPUTED_OPEN)
    {
        let mut parts = nodes(tree, at);
        if let (Some(left), Some(right)) = (parts.next(), parts.next()) {
            let left = target(tree, symbols, left);
            let right = target(tree, symbols, right);
            if left == unknown() || right == unknown() {
                return unknown();
            }
            let mark = languages::begin_name(symbols);
            languages::push_name(symbols, tree.source, left);
            languages::push_text(symbols, languages::NameText(MEMBER_SEPARATOR));
            languages::push_name(symbols, tree.source, right);
            return languages::end_name(symbols, tree.source, mark);
        }
    }
    unknown()
}
/// Import specifiers, with one unknown binding for a side-effect import.
pub(crate) fn imports<'a>(
    tree: &'a Tree<'_>,
    at: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    let specifier = move |child: &languages::Position| {
        matches!(
            kind(tree, *child),
            Kind::Node(
                JavaScriptNode::ImportSpecifier
                    | JavaScriptNode::ImportDefaultSpecifier
                    | JavaScriptNode::ImportNamespaceSpecifier
            )
        )
    };
    let empty = nodes(tree, at).find(specifier).is_none();
    nodes(tree, at)
        .filter(specifier)
        .chain(std::iter::once(at).filter(move |_| empty))
}
/// Local imported name; the last identifier distinguishes an alias from the imported name. A dynamic `import(..)` binds no name: its
/// binding is the unknown value.
pub(crate) fn import_binding(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> languages::Symbol {
    if matches!(
        kind(tree, at),
        Kind::Node(JavaScriptNode::ImportExpression | JavaScriptNode::CallExpression)
    ) {
        return unknown();
    }
    nodes(tree, at)
        .filter(|child| kind(tree, *child) == Kind::Node(JavaScriptNode::Identifier))
        .last()
        .map_or_else(unknown, |name| named(tree, symbols, name))
}
/// The independently decoded module specifier. A dynamic `import(..)` and a call of `require` name it by their first argument: a string
/// is the module, and anything else is the unknown value, since the module is not known until the program runs. Any other call
/// names none.
pub(crate) fn import_target(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    at: languages::Position,
) -> Option<languages::Symbol> {
    if kind(tree, at) == Kind::Node(JavaScriptNode::CallExpression) {
        let mut parts = nodes(tree, at);
        let callee = parts.next()?;
        if kind(tree, callee) != Kind::Node(JavaScriptNode::Identifier) || spelling(tree, callee).0 != REQUIRE
        {
            return None;
        }
        return Some(
            parts
                .next()
                .and_then(|source| string(tree, symbols, source))
                .unwrap_or_else(unknown),
        );
    }
    if kind(tree, at) == Kind::Node(JavaScriptNode::ImportExpression) {
        return Some(
            nodes(tree, at)
                .next()
                .and_then(|source| string(tree, symbols, source))
                .unwrap_or_else(unknown),
        );
    }
    nodes(tree, at).find_map(|child| string(tree, symbols, child))
}

/// The arguments the call or `new` at `call` writes, in order; the callee is no argument.
pub(crate) fn arguments_of<'a>(
    tree: &'a Tree<'_>,
    call: languages::Position,
) -> impl Iterator<Item = languages::Position> + 'a {
    nodes(tree, call).skip(1)
}

/// The name the argument at `argument` hands on: the one identifier it is, behind a spread; the empty name when it is anything else.
#[must_use]
pub(crate) fn passed(
    tree: &Tree<'_>,
    symbols: &mut languages::Symbols,
    argument: languages::Position,
) -> languages::Symbol {
    let mut current = argument;
    while kind(tree, current) == Kind::Node(JavaScriptNode::SpreadElement) {
        match nodes(tree, current).next() {
            Some(inner) => current = inner,
            None => return languages::NO_SYMBOL,
        }
    }
    if kind(tree, current) == Kind::Node(JavaScriptNode::Identifier) {
        named(tree, symbols, current)
    } else {
        languages::NO_SYMBOL
    }
}
