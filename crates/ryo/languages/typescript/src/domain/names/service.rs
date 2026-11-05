//! Names and decoded values supplied by the TypeScript syntax tree.

#![expect(
    clippy::wildcard_enum_match_arm,
    reason = "Each name operation selects the syntax shapes carrying that name; other shapes have no such static name, while schemas maps the parser vocabulary exhaustively"
)]

use super::{
    constants::{
        BLOCK_COMMENT, BLOCK_END, CLOSE_PARAMETERS, CONSTANT, EXTENDS, LINE_COMMENT, OPEN_COMPUTED,
        OPEN_PARAMETERS, SELECTION,
    },
    types::{Assignment, Reference, Spelling},
};
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::{
    service::{kind, last, parent},
    types::Kind,
};
use crate::domain::tokens::types::Token;
use ryo::languages::{self, Position, Symbol, Symbols, Tree};

/// Direct AST children, excluding source tokens and gaps.
pub(crate) fn nodes<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    languages::children(tree.elements, at)
        .filter(|child| matches!(kind(tree, *child), Kind::Node(_)))
}
/// Exact spelling of an element.
pub(crate) fn text<'a>(tree: &'a Tree<'_>, at: Position) -> Spelling<'a> {
    let start = usize::try_from(languages::start_at(tree.elements, at).0).unwrap_or(usize::MAX);
    let end = usize::try_from(languages::end_at(tree.elements, at).0).unwrap_or(usize::MAX);
    Spelling(
        std::str::from_utf8(tree.source.0.get(start..end).unwrap_or_default()).unwrap_or_default(),
    )
}
/// Intern the source range of an expression.
pub(crate) fn written(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    languages::intern_derived(symbols, tree.source, languages::NameText(text(tree, at).0))
}
/// A named value unavailable without evaluation or an anonymous binding.
pub(crate) fn unknown() -> Symbol {
    languages::unknown_symbol(languages::UnknownReason(0))
}
/// The spelling/value of an identifier's direct token.
pub(crate) fn identifier(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    if kind(tree, at) == Kind::Node(TypeScriptNode::Literal)
        && let Some(value) = string(tree, symbols, at)
    {
        return value;
    }
    let token = languages::children(tree.elements, at).find(|child| {
        matches!(
            kind(tree, *child),
            Kind::Token(Token::IdentifierToken | Token::PrivateIdentifierToken | Token::Keyword)
        )
    });
    match token {
        None => written(tree, symbols, at),
        Some(token) => languages::decoded_at(tree.elements, token).map_or_else(
            || languages::text_at(tree.elements, token),
            |value| languages::intern_derived(symbols, tree.source, value),
        ),
    }
}
/// The value of a string literal, decoded by the parser rather than inferred from its quotes.
pub(crate) fn string(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Option<Symbol> {
    let token = languages::children(tree.elements, at).find(|child| {
        matches!(
            kind(tree, *child),
            Kind::Token(Token::String | Token::JSXTextToken)
        )
    })?;
    Some(
        languages::decoded_at(tree.elements, token).map_or_else(unknown, |value| {
            languages::intern_derived(symbols, tree.source, value)
        }),
    )
}
/// A comment's content without delimiters.
pub(crate) fn comment(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    let raw = text(tree, at).0;
    let content = raw
        .strip_prefix(LINE_COMMENT)
        .or_else(|| {
            let rest = raw.strip_prefix(BLOCK_COMMENT)?;
            rest.strip_suffix(BLOCK_END)
        })
        .unwrap_or(raw);
    languages::intern_derived(symbols, tree.source, languages::NameText(content))
}
/// Find a direct delimiter without reading tokens nested inside a child expression.
pub(crate) fn token(tree: &Tree<'_>, at: Position, spelling: Spelling<'_>) -> Option<Position> {
    languages::children(tree.elements, at).find(|child| text(tree, *child).0 == spelling.0)
}
/// A declaration name precedes its parameters or base clause.
pub(crate) fn name_before(
    tree: &Tree<'_>,
    at: Position,
    before: Option<Position>,
) -> Option<Position> {
    nodes(tree, at).find(|child| {
        kind(tree, *child) == Kind::Node(TypeScriptNode::Identifier)
            && before.is_none_or(|before| {
                languages::start_at(tree.elements, *child)
                    < languages::start_at(tree.elements, before)
            })
    })
}
/// A static dotted name; computed selections and other runtime expressions remain unknown.
pub(crate) fn target(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    match kind(tree, at) {
        Kind::Node(TypeScriptNode::Identifier | TypeScriptNode::PrivateIdentifier) => identifier(tree, symbols, at),
        Kind::Node(TypeScriptNode::ThisExpression | TypeScriptNode::Super) => written(tree, symbols, at),
        Kind::Node(TypeScriptNode::MemberExpression | TypeScriptNode::TSQualifiedName) => {
            if token(tree, at, Spelling(OPEN_COMPUTED)).is_some() {
                return unknown();
            }
            let mut parts = nodes(tree, at);
            let Some(left) = parts.next() else {
                return unknown();
            };
            let Some(right) = parts.next() else {
                return unknown();
            };
            let left = target(tree, symbols, left);
            let right = target(tree, symbols, right);
            if left == unknown() || right == unknown() {
                return unknown();
            }
            let mark = languages::begin_name(symbols);
            languages::push_name(symbols, tree.source, left);
            languages::push_text(symbols, languages::NameText(SELECTION));
            languages::push_name(symbols, tree.source, right);
            languages::end_name(symbols, tree.source, mark)
        }
        Kind::Node(
            TypeScriptNode::CallExpression
            | TypeScriptNode::Decorator
            | TypeScriptNode::TSInstantiationExpression
            | TypeScriptNode::ChainExpression
            | TypeScriptNode::TSClassImplements
            | TypeScriptNode::TSInterfaceHeritage,
        ) => nodes(tree, at)
            .next()
            .map_or_else(unknown, |child| target(tree, symbols, child)),
        _ => unknown(),
    }
}
/// Parameter AST nodes between the callable's parameter delimiters.
pub(crate) fn parameters<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Position> + 'a {
    let callable = matches!(
        kind(tree, at),
        Kind::Node(
            TypeScriptNode::FunctionDeclaration
                | TypeScriptNode::FunctionExpression
                | TypeScriptNode::ArrowFunctionExpression
                | TypeScriptNode::TSDeclareFunction
                | TypeScriptNode::TSEmptyBodyFunctionExpression
                | TypeScriptNode::TSFunctionType
                | TypeScriptNode::TSConstructorType
                | TypeScriptNode::TSCallSignatureDeclaration
                | TypeScriptNode::TSConstructSignatureDeclaration
                | TypeScriptNode::TSMethodSignature
        )
    );
    callable.then_some(at).into_iter().flat_map(move |at| {
        let open = token(tree, at, Spelling(OPEN_PARAMETERS))
            .map(|child| languages::end_at(tree.elements, child));
        let mut depth = 0_i32;
        let close = languages::children(tree.elements, at)
            .filter(|child| [OPEN_PARAMETERS, CLOSE_PARAMETERS].contains(&text(tree, *child).0))
            .find(|child| {
                let opening = text(tree, *child).0 == OPEN_PARAMETERS;
                depth += if opening { 1_i32 } else { -1_i32 };
                depth == 0_i32
            })
            .map(|child| languages::start_at(tree.elements, child));
        let first = nodes(tree, at).next();
        nodes(tree, at).filter(move |child| match (open, close) {
            (Some(open), Some(close)) => {
                languages::start_at(tree.elements, *child) >= open
                    && languages::end_at(tree.elements, *child) <= close
            }
            _ => {
                kind(tree, at) == Kind::Node(TypeScriptNode::ArrowFunctionExpression) && Some(*child) == first
            }
        })
    })
}
/// The name of a simple parameter; destructuring has no single binding.
pub(crate) fn parameter_name(tree: &Tree<'_>, symbols: &mut Symbols, at: Position) -> Symbol {
    match kind(tree, at) {
        Kind::Node(TypeScriptNode::Identifier) => identifier(tree, symbols, at),
        Kind::Node(TypeScriptNode::RestElement | TypeScriptNode::AssignmentPattern | TypeScriptNode::TSParameterProperty) => {
            nodes(tree, at)
                .next()
                .map_or_else(unknown, |child| parameter_name(tree, symbols, child))
        }
        _ => unknown(),
    }
}
/// The binding positions in a pattern, without property keys or default expressions.
pub(crate) fn bindings<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Position> + 'a {
    (at.0..=last(tree, at).0)
        .map(Position)
        .filter(move |candidate| {
            if !matches!(
                kind(tree, *candidate),
                Kind::Node(TypeScriptNode::Identifier | TypeScriptNode::PrivateIdentifier | TypeScriptNode::Literal)
            ) {
                return false;
            }
            let mut child = *candidate;
            while child != at {
                let enclosing = parent(tree, child);
                let valid = match kind(tree, enclosing) {
                    Kind::Node(
                        TypeScriptNode::RestElement | TypeScriptNode::AssignmentPattern | TypeScriptNode::TSParameterProperty,
                    ) => nodes(tree, enclosing).next() == Some(child),
                    Kind::Node(TypeScriptNode::Property) => nodes(tree, enclosing).last() == Some(child),
                    Kind::Node(TypeScriptNode::ArrayPattern | TypeScriptNode::ObjectPattern) => true,
                    _ => false,
                };
                if !valid {
                    return false;
                }
                child = enclosing;
            }
            true
        })
}
/// Names owned by one declaration.
pub(crate) fn declared<'a>(
    tree: &'a Tree<'_>,
    at: Position,
) -> impl Iterator<Item = Position> + 'a {
    let mut children =
        nodes(tree, at).filter(|child| kind(tree, *child) != Kind::Node(TypeScriptNode::Decorator));
    let named = match kind(tree, at) {
        Kind::Node(TypeScriptNode::ImportSpecifier) => children.last(),
        Kind::Node(
            TypeScriptNode::FunctionDeclaration | TypeScriptNode::FunctionExpression | TypeScriptNode::TSDeclareFunction,
        ) => name_before(tree, at, token(tree, at, Spelling(OPEN_PARAMETERS))),
        Kind::Node(TypeScriptNode::ClassDeclaration | TypeScriptNode::ClassExpression) => {
            name_before(tree, at, token(tree, at, Spelling(EXTENDS)))
        }
        Kind::Node(
            TypeScriptNode::MethodDefinition
            | TypeScriptNode::PropertyDefinition
            | TypeScriptNode::AccessorProperty
            | TypeScriptNode::TSPropertySignature
            | TypeScriptNode::TSMethodSignature,
        ) if token(tree, at, Spelling(OPEN_COMPUTED)).is_some() => None,
        _ => children.next(),
    };
    named
        .into_iter()
        .flat_map(move |child| bindings(tree, child))
}
/// The common category of a TypeScript declaration.
pub(crate) fn declaration_kind(tree: &Tree<'_>, at: Position) -> languages::DeclarationKind {
    use languages::DeclarationKind as Declaration;
    match kind(tree, at) {
        Kind::Node(TypeScriptNode::FunctionDeclaration | TypeScriptNode::TSDeclareFunction) => Declaration::Function,
        Kind::Node(TypeScriptNode::ClassDeclaration) => Declaration::Class,
        Kind::Node(TypeScriptNode::TSInterfaceDeclaration) => Declaration::Interface,
        Kind::Node(TypeScriptNode::TSTypeAliasDeclaration) => Declaration::TypeAlias,
        Kind::Node(TypeScriptNode::TSTypeParameter) => Declaration::TypeParameter,
        Kind::Node(TypeScriptNode::TSEnumDeclaration) => Declaration::Enum,
        Kind::Node(TypeScriptNode::TSEnumMember) => Declaration::EnumMember,
        Kind::Node(TypeScriptNode::TSModuleDeclaration) => Declaration::Namespace,
        Kind::Node(
            TypeScriptNode::ImportSpecifier | TypeScriptNode::ImportDefaultSpecifier | TypeScriptNode::ImportNamespaceSpecifier,
        ) => Declaration::Import,
        Kind::Node(TypeScriptNode::MethodDefinition | TypeScriptNode::TSMethodSignature) => Declaration::Method,
        Kind::Node(TypeScriptNode::LabeledStatement) => Declaration::Label,
        Kind::Node(
            TypeScriptNode::PropertyDefinition | TypeScriptNode::AccessorProperty | TypeScriptNode::TSPropertySignature,
        ) => Declaration::Property,
        Kind::Node(TypeScriptNode::VariableDeclarator)
            if languages::children(tree.elements, parent(tree, at)).any(|child| {
                kind(tree, child) == Kind::Token(Token::Keyword) && text(tree, child).0 == CONSTANT
            }) =>
        {
            Declaration::Constant
        }
        _ => Declaration::Variable,
    }
}
/// A reference excludes binding names and static property keys, but includes value expressions.
pub(crate) fn referenced(tree: &Tree<'_>, at: Position) -> Reference {
    let enclosing = parent(tree, at);
    let parent_kind = kind(tree, enclosing);
    if matches!(
        parent_kind,
        Kind::Node(
            TypeScriptNode::ImportSpecifier
                | TypeScriptNode::ImportDefaultSpecifier
                | TypeScriptNode::ImportNamespaceSpecifier
                | TypeScriptNode::MemberExpression
                | TypeScriptNode::TSQualifiedName
        )
    ) {
        return Reference::Binding;
    }
    if matches!(parent_kind, Kind::Node(TypeScriptNode::Property))
        && nodes(tree, enclosing).last() != Some(at)
        && token(tree, enclosing, Spelling(OPEN_COMPUTED)).is_none()
    {
        return Reference::Binding;
    }
    let mut ancestor = enclosing;
    loop {
        if matches!(
            kind(tree, ancestor),
            Kind::Node(
                TypeScriptNode::VariableDeclarator
                    | TypeScriptNode::FunctionDeclaration
                    | TypeScriptNode::FunctionExpression
                    | TypeScriptNode::TSDeclareFunction
                    | TypeScriptNode::ClassDeclaration
                    | TypeScriptNode::ClassExpression
                    | TypeScriptNode::TSTypeAliasDeclaration
                    | TypeScriptNode::TSInterfaceDeclaration
                    | TypeScriptNode::TSEnumDeclaration
                    | TypeScriptNode::TSModuleDeclaration
                    | TypeScriptNode::TSTypeParameter
                    | TypeScriptNode::MethodDefinition
                    | TypeScriptNode::PropertyDefinition
                    | TypeScriptNode::AccessorProperty
                    | TypeScriptNode::TSPropertySignature
                    | TypeScriptNode::TSMethodSignature
                    | TypeScriptNode::TSEnumMember
            )
        ) && declared(tree, ancestor).any(|binding| binding == at)
        {
            return Reference::Binding;
        }
        if parameters(tree, ancestor)
            .any(|parameter| bindings(tree, parameter).any(|binding| binding == at))
        {
            return Reference::Binding;
        }
        if ancestor == parent(tree, ancestor) {
            break;
        }
        ancestor = parent(tree, ancestor);
    }
    Reference::Use
}
/// Direct string assignment pairs.
pub(crate) fn assignment(tree: &Tree<'_>, at: Position) -> Option<Assignment> {
    let mut children = nodes(tree, at);
    let binding = children.next()?;
    let value = children.last()?;
    let token = languages::children(tree.elements, value)
        .any(|child| kind(tree, child) == Kind::Token(Token::String));
    token.then_some(Assignment { binding, value })
}
/// The explicit bases of a class or interface.
pub(crate) fn bases<'a>(tree: &'a Tree<'_>, at: Position) -> impl Iterator<Item = Position> + 'a {
    let interface = kind(tree, at) == Kind::Node(TypeScriptNode::TSInterfaceDeclaration);
    let extends =
        token(tree, at, Spelling(EXTENDS)).map(|child| languages::end_at(tree.elements, child));
    nodes(tree, at).filter(move |child| {
        if interface {
            return kind(tree, *child) == Kind::Node(TypeScriptNode::TSInterfaceHeritage);
        }
        matches!(kind(tree, *child), Kind::Node(TypeScriptNode::TSClassImplements))
            || (extends.is_some_and(|from| languages::start_at(tree.elements, *child) >= from)
                && !matches!(
                    kind(tree, *child),
                    Kind::Node(TypeScriptNode::ClassBody | TypeScriptNode::TSTypeParameterInstantiation)
                ))
    })
}
