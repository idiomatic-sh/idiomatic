//! The projection of the adapter: annotate, normalized, attach, jsx_names and more.

use super::super::constants::{
    COMMONJS_EXTENSION, JSX_EXTENSION, MODULE_EXTENSION, PLAIN_EXTENSION,
};
use super::super::types::{
    ChildCursor, Decoded, DecodedText, Piece, Text, VisitDepth, VisitorStack,
};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::{Dialect, Kind};
use crate::domain::tokens::types::Token;
use oxc_ast::ast::FormalParameter;
use oxc_span::{GetSpan as _, SourceType, Span};
use ryo::languages::{self, Offset};

/// `ESTree` stores a binding's annotation inside the binding, including its range.
pub(in super::super) fn annotate(piece: &mut Piece) {
    if let Some(index) = piece
        .children
        .iter()
        .position(|child| child.kind == Kind::Node(Node::TSTypeAnnotation))
    {
        let annotation = piece.children.remove(index);
        if let Some(binding) = piece
            .children
            .iter_mut()
            .find(|child| child.kind != Kind::Node(Node::Decorator))
        {
            binding.span.end = annotation.span.end;
            binding.children.push(annotation);
        }
    }
}

/// Normalize parser wrappers whose children belong to a standard node.
/// Finish a node only after its typed visitor has normalized the native form.
pub(in super::super) fn normalized(
    projection: &mut VisitorStack,
    visit: impl FnOnce(&mut VisitorStack),
    normalize: impl FnOnce(&mut Piece),
) {
    let previous = projection
        .deferred
        .replace(VisitDepth(projection.open.len()));
    visit(projection);
    projection.deferred = previous;
    if let Some(mut piece) = projection.open.pop() {
        normalize(&mut piece);
        attach(projection, piece);
    }
}

/// Preserve source ordering while attaching a completed standard node or transparent wrapper.
pub(in super::super) fn attach(projection: &mut VisitorStack, mut piece: Piece) {
    piece
        .children
        .sort_by_key(|child| (child.span.start, std::cmp::Reverse(child.span.end)));
    let pieces = if matches!(piece.kind, Kind::Node(_)) {
        vec![piece]
    } else {
        piece.children
    };
    match projection.open.last_mut() {
        Some(parent) => parent.children.extend(pieces),
        None => projection.roots.extend(pieces),
    }
}

/// JSX names have their own standard identity even where Oxc uses an identifier child.
pub(in super::super) fn jsx_names(piece: &mut Piece) {
    for child in &mut piece.children {
        if child.kind == Kind::Node(Node::Identifier) {
            child.kind = Kind::Node(Node::JSXIdentifier);
        }
    }
}

/// Bind annotations, decorators and default values to the standard parameter node.
pub(in super::super) fn parameter(piece: &mut Piece, parameter: &FormalParameter<'_>) {
    annotate(piece);
    let decorators: Vec<_> = piece
        .children
        .extract_if(.., |child| child.kind == Kind::Node(Node::Decorator))
        .collect();
    if parameter.optional
        && parameter.type_annotation.is_none()
        && let Some(binding) = piece.children.first_mut()
    {
        binding.span.end = parameter
            .initializer
            .as_ref()
            .map_or(parameter.span.end, |_| parameter.pattern.span().end + 1);
    }
    let property = parameter.accessibility.is_some() || parameter.readonly || parameter.r#override;
    let whole = piece.span;
    if parameter.initializer.is_some() {
        piece.kind = Kind::Node(Node::AssignmentPattern);
        piece.span.start = parameter.pattern.span().start;
    }
    if property {
        let argument = std::mem::replace(piece, leaf(Kind::Node(Node::TSParameterProperty), whole));
        if matches!(argument.kind, Kind::Node(_)) {
            piece.children.push(argument);
        } else {
            piece.children.extend(argument.children);
        }
        piece.children.extend(decorators);
    } else if parameter.initializer.is_some() {
        piece.children.extend(decorators);
    } else if let Some(binding) = piece.children.first_mut() {
        binding.children.extend(decorators);
    } else {
        // A parameter with no binding child has no node to hold its decorators; the parser gives none such.
    }
}

/// Expression-bodied arrows omit the parser's block and expression-statement wrappers.
pub(in super::super) fn expression_body(piece: &mut Piece) {
    if let Some(index) = piece
        .children
        .iter()
        .position(|child| child.kind == Kind::Node(Node::BlockStatement))
    {
        let block = piece.children.remove(index);
        for statement in block.children {
            piece.children.extend(statement.children);
        }
    }
}

/// Put a token under the first deepest AST node containing its range.
pub(in super::super) fn insert(node: &mut Piece, mut token: Piece) {
    if node.kind == Kind::Node(Node::JSXAttribute)
        && token.kind == Kind::Token(Token::String)
        && node
            .children
            .iter()
            .any(|child| child.kind == Kind::Node(Node::Literal) && child.span == token.span)
    {
        token.kind = Kind::Token(Token::JSXTextToken);
    }
    while let Some(child) = node.children.get_mut(node.cursor.0) {
        if child.kind == Kind::Node(Node::TemplateElement)
            && token.kind == Kind::Token(Token::Template)
            && token.span.start <= child.span.start
            && child.span.end <= token.span.end
        {
            child.span = token.span;
        }
        if matches!(child.kind, Kind::Node(_))
            && child.span.start <= token.span.start
            && child.span.end >= token.span.end
        {
            insert(child, token);
            return;
        }
        if matches!(child.kind, Kind::Node(_)) && child.span.start > token.span.start {
            break;
        }
        node.cursor.0 += 1;
    }
    if node.kind == Kind::Node(Node::JSXIdentifier)
        && matches!(
            token.kind,
            Kind::Token(Token::IdentifierToken | Token::Keyword)
        )
    {
        token.kind = Kind::Token(Token::JSXIdentifierToken);
    }
    // A keyword the parser decoded no name for that the standard holds as an identifier is its identifier token: a function's
    // `this` parameter. A key of import options is decoded, and stays the keyword the standard says it is.
    if node.kind == Kind::Node(Node::Identifier)
        && token.kind == Kind::Token(Token::Keyword)
        && matches!(token.text, Text::Source)
    {
        token.kind = Kind::Token(Token::IdentifierToken);
    }
    node.children.push(token);
}

/// Emit the standard tree, preserving AST parentage and source order.
pub(in super::super) fn write(node: &mut Piece, builder: &mut languages::TableBuilder<'_>) {
    node.children
        .sort_by_key(|child| (child.span.start, std::cmp::Reverse(child.span.end)));
    let (start, end) = (Offset(node.span.start), Offset(node.span.end));
    match node.kind {
        Kind::Node(_) => {
            languages::enter_node(builder, kind_bits(node.kind), start);
            for child in &mut node.children {
                write(child, builder);
            }
            languages::leave_node(builder, end);
        }
        Kind::Token(_) | Kind::Extra(_) | Kind::Pattern(_) => match &node.text {
            Text::Source => languages::token(builder, kind_bits(node.kind), start, end),
            Text::Decoded(text) => languages::decoded_token(
                builder,
                kind_bits(node.kind),
                start,
                end,
                languages::NameText(&text.0),
            ),
        },
    }
}

/// The meta and the property of a meta property at `span` (`import.meta`, `new.target`): the identifiers TypeScript-ESTree gives it,
/// the first word at its start and the second at its end, each decoded as the parser does an identifier. Oxc keeps neither as a node.
pub(in super::super) fn meta_words(
    projection: &mut VisitorStack,
    span: Span,
    words: [languages::NameText<'static>; 2],
) -> [Piece; 2] {
    let [meta, property] = words;
    let meta_end = span.start + u32::try_from(meta.0.len()).unwrap_or(0);
    let property_start = span.end - u32::try_from(property.0.len()).unwrap_or(0);
    let spans = [
        Span::new(span.start, meta_end),
        Span::new(property_start, span.end),
    ];
    for (word, word_span) in words.into_iter().zip(spans) {
        projection.decoded.push(Decoded {
            span: word_span,
            value: DecodedText(word.0.to_owned()),
            token_kind: Token::IdentifierToken,
        });
    }
    spans.map(|word_span| leaf(Kind::Node(Node::Identifier), word_span))
}

/// A token or gap with the source spelling.
pub(in super::super) fn leaf(kind: Kind, span: Span) -> Piece {
    Piece {
        kind,
        span,
        children: Vec::new(),
        text: Text::Source,
        cursor: ChildCursor(0),
    }
}

/// Preserve unlexed bytes as whitespace only when ECMAScript classifies them that way.
pub(in super::super) fn gap(
    root: &mut Piece,
    text: languages::NameText<'_>,
    start: Offset,
    end: Offset,
) {
    let (text, start, end) = (text.0, start.0, end.0);
    let Some(source) = text.get(
        usize::try_from(start).unwrap_or(usize::MAX)..usize::try_from(end).unwrap_or(usize::MAX),
    ) else {
        return;
    };
    let mut from = start;
    let mut previous = None;
    for (index, character) in source.char_indices() {
        let kind = if character.is_whitespace() || character == '\u{feff}' {
            Extra::Whitespace
        } else {
            Extra::Error
        };
        let at = start + u32::try_from(index).unwrap_or(u32::MAX);
        if let Some(before) = previous
            && before != kind
        {
            insert(root, leaf(Kind::Extra(before), Span::new(from, at)));
            from = at;
        }
        previous = Some(kind);
    }
    if let Some(kind) = previous {
        insert(root, leaf(Kind::Extra(kind), Span::new(from, end)));
    }
}

/// What Oxc reads a file of `dialect` as: what it reads a file of that extension as, and a file of declarations as declarations.
pub(in super::super) fn source_type_of(dialect: Dialect) -> SourceType {
    let (extension, declarations) = match dialect {
        Dialect::TypeScript => (PLAIN_EXTENSION, false),
        Dialect::Tsx => (JSX_EXTENSION, false),
        Dialect::Module => (MODULE_EXTENSION, false),
        Dialect::CommonJs => (COMMONJS_EXTENSION, false),
        Dialect::Declarations => (PLAIN_EXTENSION, true),
        Dialect::ModuleDeclarations => (MODULE_EXTENSION, true),
        Dialect::CommonJsDeclarations => (COMMONJS_EXTENSION, true),
    };
    SourceType::from_extension(extension)
        .unwrap_or_else(|_| SourceType::ts())
        .with_typescript_definition(declarations)
}
