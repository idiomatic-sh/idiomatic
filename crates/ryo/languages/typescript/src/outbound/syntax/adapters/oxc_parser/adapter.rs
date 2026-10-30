// ignore: idiomatic.arch/file-length - Typed projection, token attachment and diagnostics share one borrowed parser result and preserve the same walk; splitting this operation would obscure its source-order invariant. Wire-kind conversion is already separate in schemas.rs, and the adapter spec places no other file for logic beside adapter.rs.
//! Read TypeScript with Oxc and project its typed AST into TypeScript-ESTree nodes.

use oxc_allocator::Allocator;
use oxc_ast::{
    AstKind,
    ast::{
        ArrowFunctionExpression, FormalParameter, FormalParameterRest, ImportMeta, JSXAttribute,
        JSXClosingElement, JSXMemberExpression, JSXNamespacedName, JSXOpeningElement, NewTarget,
        TSGlobalDeclaration, VariableDeclarator,
    },
};
use oxc_ast_visit::{Visit, walk};
use oxc_parser::{ParseOptions, Parser, config::TokensParserConfig};
use oxc_span::{GetSpan as _, SourceType, Span};
use ryo::languages::{self, Offset};

use super::constants::{
    COMMONJS_EXTENSION, IMPORT_META, IMPORT_OPTIONS_KEY, JSX_EXTENSION, MODULE_EXTENSION,
    NEW_TARGET, NOT_UTF8, PLAIN_EXTENSION,
};
use super::schemas::{node_kind, token_kind};
use super::types::{ChildCursor, Decoded, DecodedText, Piece, Text, VisitDepth, VisitorStack};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::{Dialect, Kind};
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

/// The Oxc syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxOxcParserAdapter;

impl<'a> Visit<'a> for VisitorStack {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        if let Some(value) = kind.identifier_name() {
            let import_options = value == IMPORT_OPTIONS_KEY
                && self.open.iter().rev().take(3).map(|piece| piece.kind).eq([
                    Kind::Node(Node::Property),
                    Kind::Node(Node::ObjectExpression),
                    Kind::Node(Node::TSImportType),
                ]);
            self.decoded.push(Decoded {
                span: kind.span(),
                value: DecodedText(value.to_string()),
                token_kind: if import_options {
                    Token::Keyword
                } else {
                    Token::IdentifierToken
                },
            });
        }
        if let Some(value) = kind.as_private_identifier() {
            self.decoded.push(Decoded {
                span: value.span,
                value: DecodedText(value.name.to_string()),
                token_kind: Token::PrivateIdentifierToken,
            });
        }
        if let AstKind::StringLiteral(literal) = kind {
            self.decoded.push(Decoded {
                span: literal.span,
                value: DecodedText(literal.value.to_string()),
                token_kind: Token::String,
            });
        }
        self.open.push(Piece {
            kind: node_kind(kind).map_or(Kind::Extra(Extra::Error), Kind::Node),
            span: kind.span(),
            children: Vec::new(),
            text: Text::Source,
            cursor: ChildCursor(0),
        });
    }

    fn visit_jsx_attribute(&mut self, it: &JSXAttribute<'a>) {
        normalized(
            self,
            |projection| walk::walk_jsx_attribute(projection, it),
            jsx_names,
        );
    }
    fn visit_jsx_opening_element(&mut self, it: &JSXOpeningElement<'a>) {
        normalized(
            self,
            |projection| walk::walk_jsx_opening_element(projection, it),
            jsx_names,
        );
    }
    fn visit_jsx_closing_element(&mut self, it: &JSXClosingElement<'a>) {
        normalized(
            self,
            |projection| walk::walk_jsx_closing_element(projection, it),
            jsx_names,
        );
    }
    fn visit_jsx_member_expression(&mut self, it: &JSXMemberExpression<'a>) {
        normalized(
            self,
            |projection| walk::walk_jsx_member_expression(projection, it),
            jsx_names,
        );
    }
    fn visit_jsx_namespaced_name(&mut self, it: &JSXNamespacedName<'a>) {
        normalized(
            self,
            |projection| walk::walk_jsx_namespaced_name(projection, it),
            jsx_names,
        );
    }
    fn visit_import_meta(&mut self, it: &ImportMeta) {
        let words = meta_words(self, it.span, IMPORT_META);
        normalized(
            self,
            |projection| walk::walk_import_meta(projection, it),
            |piece| piece.children.extend(words),
        );
    }
    fn visit_new_target(&mut self, it: &NewTarget) {
        let words = meta_words(self, it.span, NEW_TARGET);
        normalized(
            self,
            |projection| walk::walk_new_target(projection, it),
            |piece| piece.children.extend(words),
        );
    }
    fn visit_ts_global_declaration(&mut self, it: &TSGlobalDeclaration<'a>) {
        normalized(
            self,
            |projection| walk::walk_ts_global_declaration(projection, it),
            |piece| {
                piece
                    .children
                    .push(leaf(Kind::Node(Node::Identifier), it.global_span));
            },
        );
    }
    fn visit_variable_declarator(&mut self, it: &VariableDeclarator<'a>) {
        normalized(
            self,
            |projection| walk::walk_variable_declarator(projection, it),
            annotate,
        );
    }
    fn visit_formal_parameter(&mut self, it: &FormalParameter<'a>) {
        normalized(
            self,
            |projection| walk::walk_formal_parameter(projection, it),
            |piece| parameter(piece, it),
        );
    }
    fn visit_formal_parameter_rest(&mut self, it: &FormalParameterRest<'a>) {
        normalized(
            self,
            |projection| walk::walk_formal_parameter_rest(projection, it),
            |piece| {
                annotate(piece);
                piece.kind = Kind::Extra(Extra::Error);
            },
        );
    }
    fn visit_arrow_function_expression(&mut self, it: &ArrowFunctionExpression<'a>) {
        if it.expression {
            normalized(
                self,
                |projection| walk::walk_arrow_function_expression(projection, it),
                expression_body,
            );
        } else {
            walk::walk_arrow_function_expression(self, it);
        }
    }
    fn leave_node(&mut self, _node: AstKind<'a>) {
        if self
            .deferred
            .as_ref()
            .is_some_and(|depth| self.open.len() == depth.0 + 1)
        {
            return;
        }
        if let Some(piece) = self.open.pop() {
            attach(self, piece);
        }
    }
}

/// `ESTree` stores a binding's annotation inside the binding, including its range.
fn annotate(piece: &mut Piece) {
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
fn normalized(
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
fn attach(projection: &mut VisitorStack, mut piece: Piece) {
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
fn jsx_names(piece: &mut Piece) {
    for child in &mut piece.children {
        if child.kind == Kind::Node(Node::Identifier) {
            child.kind = Kind::Node(Node::JSXIdentifier);
        }
    }
}

/// Bind annotations, decorators and default values to the standard parameter node.
fn parameter(piece: &mut Piece, parameter: &FormalParameter<'_>) {
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
fn expression_body(piece: &mut Piece) {
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
fn insert(node: &mut Piece, mut token: Piece) {
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
fn write(node: &mut Piece, builder: &mut languages::TableBuilder<'_>) {
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
        Kind::Token(_) | Kind::Extra(_) => match &node.text {
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
fn meta_words(
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
fn leaf(kind: Kind, span: Span) -> Piece {
    Piece {
        kind,
        span,
        children: Vec::new(),
        text: Text::Source,
        cursor: ChildCursor(0),
    }
}

/// Preserve unlexed bytes as whitespace only when ECMAScript classifies them that way.
fn gap(root: &mut Piece, text: languages::NameText<'_>, start: Offset, end: Offset) {
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
fn source_type_of(dialect: Dialect) -> SourceType {
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

impl SyntaxPort for SyntaxOxcParserAdapter {
    #[expect(
        clippy::too_many_lines,
        reason = "The parser result, tokens and diagnostics share one borrowed parse lifetime; this operation keeps their projection at that boundary"
    )]
    fn read(
        &self,
        text: SourceText<'_>,
        dialect: Dialect,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        let source = languages::readable_text(languages::ByteSlice(text.0)).0;
        let allocator = Allocator::default();
        let source_type = source_type_of(dialect);
        let parsed = Parser::new(&allocator, source, source_type)
            .with_options(ParseOptions {
                preserve_parens: false,
                ..ParseOptions::default()
            })
            .with_config(TokensParserConfig)
            .parse();
        let mut projection = VisitorStack::default();
        projection.visit_program(&parsed.program);
        projection
            .decoded
            .sort_by_key(|decoded| (decoded.span.start, decoded.span.end));
        let length = u32::try_from(text.0.len()).unwrap_or(u32::MAX);
        let mut root = projection.roots.remove(0);
        root.span = Span::new(0, length);
        let mut leaves = Vec::new();
        for token in &parsed.tokens {
            if token.start() < token.end() {
                let span = Span::new(token.start(), token.end());
                let mapped = match token_kind(token.kind()) {
                    Ok(token) => Kind::Token(token),
                    Err(unrecognized) => {
                        languages::diagnose(
                            builder,
                            languages::ReportText(&format!(
                                "unrecognized parser token: {unrecognized:?}"
                            )),
                            Offset(span.start),
                            Offset(span.end),
                        );
                        Kind::Extra(Extra::Error)
                    }
                };
                let mut piece = leaf(mapped, span);
                let found = match projection
                    .decoded
                    .binary_search_by_key(&(span.start, span.end), |decoded| {
                        (decoded.span.start, decoded.span.end)
                    }) {
                    Ok(index) => projection.decoded.get(index),
                    // The search answers where the span would go: it holds no decoded value.
                    Err(_) => None,
                };
                if let Some(decoded) = found {
                    piece.kind = Kind::Token(decoded.token_kind);
                    piece.text = Text::Decoded(decoded.value.clone());
                }
                leaves.push(piece);
            }
        }
        if let Some(hashbang) = &parsed.program.hashbang {
            leaves.push(leaf(Kind::Extra(Extra::Hashbang), hashbang.span));
        }
        for comment in &parsed.program.comments {
            leaves.push(leaf(
                Kind::Token(if comment.is_line() {
                    Token::Line
                } else {
                    Token::Block
                }),
                comment.span,
            ));
        }
        leaves.sort_by_key(|piece| piece.span.start);
        let mut end = 0;
        for token in leaves {
            if token.span.start > end {
                gap(
                    &mut root,
                    languages::NameText(source),
                    Offset(end),
                    Offset(token.span.start),
                );
            }
            end = token.span.end;
            insert(&mut root, token);
        }
        let readable = u32::try_from(source.len()).unwrap_or(u32::MAX);
        if end < readable {
            gap(
                &mut root,
                languages::NameText(source),
                Offset(end),
                Offset(readable),
            );
        }
        if readable < length {
            insert(
                &mut root,
                leaf(Kind::Extra(Extra::Error), Span::new(readable, length)),
            );
        }
        write(&mut root, builder);
        let semantic = oxc_semantic::SemanticBuilder::new()
            .with_check_syntax_error(true)
            .build(&parsed.program);
        for diagnostic in parsed.diagnostics.iter().chain(semantic.diagnostics.iter()) {
            let span = diagnostic
                .labels
                .as_slice()
                .first()
                .map_or(Span::new(0, readable), |label| {
                    Span::new(label.offset(), label.offset().saturating_add(label.len()))
                });
            languages::diagnose(
                builder,
                languages::ReportText(&diagnostic.to_string()),
                Offset(span.start),
                Offset(span.end),
            );
        }
        if readable < length {
            languages::diagnose(
                builder,
                languages::ReportText(NOT_UTF8),
                Offset(readable),
                Offset(length),
            );
        }
    }
}
