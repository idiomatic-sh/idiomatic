//! Oxc's typed JavaScript AST, projected to `ESTree` and merged with lexical leaves.
use super::{
    constants::{NOT_UTF8, SPREAD_CHILD},
    schemas::node_kind,
    types::{ChildCursor, Decoded, DecodedText, Element, VisitorStack},
};
use crate::domain::syntax::{
    service::kind_bits,
    types::{Goal, Kind},
};
use crate::domain::{extras::types::Extra, nodes::types::Node, tokens::types::Token};
use crate::outbound::syntax::{interface::SyntaxPort, types::SourceText};
use oxc_allocator::Allocator;
use oxc_ast::AstKind;
use oxc_ast_visit::Visit;
use oxc_parser::{ParseOptions, Parser, config::TokensParserConfig};
use oxc_span::{GetSpan as _, SourceType, Span};
use ryo::languages::{self, Offset};

/// The real parser behind the language's syntax port.
#[derive(Debug)]
pub(crate) struct SyntaxOxcParserAdapter;

impl<'a> Visit<'a> for VisitorStack {
    fn enter_node(&mut self, kind: AstKind<'a>) {
        if let AstKind::StringLiteral(string) = kind {
            self.strings.push(Decoded {
                span: string.span,
                value: DecodedText(string.value.to_string()),
            });
        }
        let mut identifier = kind
            .identifier_name()
            .map(|name| (kind.span(), name.as_str()));
        if let AstKind::PrivateIdentifier(value) = kind {
            identifier = Some((value.span, value.name.as_str()));
        }
        if let Some((span, value)) = identifier {
            self.strings.push(Decoded {
                span,
                value: DecodedText(value.to_owned()),
            });
        }
        self.stack.push(Element {
            kind: node_kind(kind).map(Kind::Node),
            span: kind.span(),
            children: Vec::new(),
            next_child: ChildCursor::default(),
            decoded: None,
        });
    }
    fn leave_node(&mut self, kind: AstKind<'a>) {
        let Some(mut element) = self.stack.pop() else {
            return;
        };
        let mut meta_length = None;
        if let AstKind::ImportMeta(_) = kind {
            meta_length = Some(6);
        }
        if let AstKind::NewTarget(_) = kind {
            meta_length = Some(3);
        }
        if let Some(length) = meta_length {
            let start = element.span.start;
            let end = element.span.end;
            element.children = vec![
                leaf(
                    Kind::Node(Node::Identifier),
                    Offset(start),
                    Offset(start + length),
                ),
                leaf(
                    Kind::Node(Node::Identifier),
                    Offset(start + length + 1),
                    Offset(end),
                ),
            ];
        }
        if let AstKind::ArrowFunctionExpression(function) = kind
            && function.expression
            && let Some(index) = element
                .children
                .iter()
                .position(|child| child.kind == Some(Kind::Node(Node::BlockStatement)))
        {
            let block = element.children.remove(index);
            for returned in block.children {
                element.children.extend(returned.children);
            }
        }
        if matches!(
            kind,
            AstKind::JSXOpeningElement(_)
                | AstKind::JSXClosingElement(_)
                | AstKind::JSXMemberExpression(_)
        ) {
            for child in &mut element.children {
                if matches!(
                    child.kind,
                    Some(Kind::Node(Node::Identifier | Node::ThisExpression))
                ) {
                    child.kind = Some(Kind::Node(Node::JSXIdentifier));
                }
            }
        }
        element
            .children
            .sort_by_key(|child| (child.span.start, std::cmp::Reverse(child.span.end)));
        let mut children = if element.kind.is_some() {
            vec![element]
        } else {
            element.children
        };
        if let Some(parent) = self.stack.last_mut() {
            parent.children.append(&mut children);
        } else {
            self.roots.append(&mut children);
        }
    }
}

/// The ECMAScript lexical category of a parser token.
fn category(token: oxc_parser::Kind) -> Token {
    use oxc_parser::Kind as Lexeme;
    if token == Lexeme::Str {
        return Token::String;
    }
    if token == Lexeme::RegExp {
        return Token::RegularExpression;
    }
    if token == Lexeme::PrivateIdentifier {
        return Token::PrivateIdentifier;
    }
    if token == Lexeme::JSXText {
        return Token::JSXText;
    }
    if matches!(
        token,
        Lexeme::NoSubstitutionTemplate
            | Lexeme::TemplateHead
            | Lexeme::TemplateMiddle
            | Lexeme::TemplateTail
    ) {
        return Token::Template;
    }
    if token.is_number() {
        return Token::Numeric;
    }
    if matches!(
        token,
        Lexeme::Await
            | Lexeme::Yield
            | Lexeme::Let
            | Lexeme::Static
            | Lexeme::Implements
            | Lexeme::Interface
            | Lexeme::Package
            | Lexeme::Private
            | Lexeme::Protected
            | Lexeme::Public
    ) {
        return Token::Identifier;
    }
    if token.is_reserved_keyword() {
        return Token::Keyword;
    }
    if token.is_identifier_or_keyword() {
        return Token::Identifier;
    }
    Token::Punctuator
}

/// Place a lexical leaf under its deepest containing AST node, retaining duplicate-range siblings.
fn insert(parent: &mut Element, leaf: Element) {
    while parent
        .children
        .get(parent.next_child.0)
        .is_some_and(|child| child.span.end <= leaf.span.start)
    {
        parent.next_child.0 += 1;
    }
    if let Some(child) = parent
        .children
        .get_mut(parent.next_child.0..)
        .and_then(|rest| {
            rest.iter_mut()
                .take_while(|child| child.span.start <= leaf.span.start)
                .find(|child| {
                    matches!(child.kind, Some(Kind::Node(_)))
                        && child.span.start <= leaf.span.start
                        && child.span.end >= leaf.span.end
                })
        })
    {
        insert(child, leaf);
    } else {
        let mut leaf = leaf;
        if parent.kind == Some(Kind::Node(Node::JSXIdentifier))
            && matches!(
                leaf.kind,
                Some(Kind::Token(Token::Identifier | Token::Keyword))
            )
        {
            leaf.kind = Some(Kind::Token(Token::JSXIdentifier));
        }
        parent.children.push(leaf);
    }
}

/// Emit a complete projected subtree into the engine's existing table builder.
fn emit(node: &mut Element, builder: &mut languages::TableBuilder<'_>) {
    node.children
        .sort_by_key(|child| (child.span.start, std::cmp::Reverse(child.span.end)));
    let (start, end) = (Offset(node.span.start), Offset(node.span.end));
    match node.kind {
        Some(Kind::Node(_)) => {
            // Oxc reads a JSX spread child; the `ESTree` JSX extension (acorn-jsx) refuses one.
            if node.kind == Some(Kind::Node(Node::JSXSpreadChild)) {
                languages::diagnose(builder, languages::ReportText(SPREAD_CHILD), start, end);
            }
            languages::enter_node(
                builder,
                kind_bits(node.kind.unwrap_or(Kind::Extra(Extra::Error))),
                start,
            );
            for child in &mut node.children {
                emit(child, builder);
            }
            languages::leave_node(builder, end);
        }
        Some(kind) => match &node.decoded {
            Some(value) => languages::decoded_token(
                builder,
                kind_bits(kind),
                start,
                end,
                languages::NameText(&value.0),
            ),
            None => languages::token(builder, kind_bits(kind), start, end),
        },
        None => {}
    }
}

/// A lossless lexical leaf, using source spelling unless the parser supplies a decoded value.
fn leaf(kind: Kind, start: Offset, end: Offset) -> Element {
    Element {
        kind: Some(kind),
        span: Span::new(start.0, end.0),
        children: Vec::new(),
        next_child: ChildCursor::default(),
        decoded: None,
    }
}

/// Insert lexemes and preserve uncovered bytes as whitespace or invalid UTF-8 extras.
fn merge_lexemes(root: &mut Element, mut leaves: Vec<Element>, readable: Offset, length: Offset) {
    let (readable, length) = (readable.0, length.0);
    leaves.sort_by_key(|entry| entry.span.start);
    let mut end = 0;
    for token in leaves {
        if end < token.span.start {
            insert(
                root,
                leaf(
                    Kind::Extra(Extra::Whitespace),
                    Offset(end),
                    Offset(token.span.start),
                ),
            );
        }
        end = token.span.end;
        insert(root, token);
    }
    if end < readable {
        insert(
            root,
            leaf(
                Kind::Extra(Extra::Whitespace),
                Offset(end),
                Offset(readable),
            ),
        );
    }
    if readable < length {
        insert(
            root,
            leaf(Kind::Extra(Extra::Error), Offset(readable), Offset(length)),
        );
    }
}

impl SyntaxPort for SyntaxOxcParserAdapter {
    fn read(&self, text: SourceText<'_>, goal: Goal, builder: &mut languages::TableBuilder<'_>) {
        let source = languages::readable_text(languages::ByteSlice(text.0)).0;
        let allocator = Allocator::default();
        let parsed = Parser::new(
            &allocator,
            source,
            match goal {
                Goal::Module => SourceType::mjs(),
                Goal::Script => SourceType::cjs().with_script(true),
                Goal::Detected => SourceType::unambiguous().with_jsx(true),
            },
        )
        .with_options(ParseOptions {
            preserve_parens: false,
            ..ParseOptions::default()
        })
        .with_config(TokensParserConfig)
        .parse();
        let checked = oxc_semantic::SemanticBuilder::new_compiler().build(&parsed.program);
        let mut projection = VisitorStack::default();
        projection.visit_program(&parsed.program);
        let length = u32::try_from(text.0.len()).unwrap_or(u32::MAX);
        let readable = u32::try_from(source.len()).unwrap_or(u32::MAX);
        let Some(mut root) = projection.roots.pop() else {
            return;
        };
        root.span = Span::new(0, length);
        projection
            .strings
            .sort_by_key(|entry| (entry.span.start, entry.span.end));
        let mut decoded = projection.strings.iter().peekable();
        let mut leaves = Vec::new();
        for token in &parsed.tokens {
            if token.start() == token.end() {
                continue;
            }
            let mut value = leaf(
                Kind::Token(category(token.kind())),
                Offset(token.start()),
                Offset(token.end()),
            );
            while decoded
                .next_if(|entry| entry.span.start < value.span.start)
                .is_some()
            {}
            value.decoded = decoded
                .peek()
                .filter(|entry| entry.span == value.span)
                .map(|entry| entry.value.clone());
            if value.decoded.is_some() && value.kind == Some(Kind::Token(Token::Keyword)) {
                value.kind = Some(Kind::Token(Token::Identifier));
            }
            leaves.push(value);
        }
        if let Some(hashbang) = &parsed.program.hashbang {
            leaves.push(leaf(
                Kind::Token(Token::Line),
                Offset(hashbang.span.start),
                Offset(hashbang.span.end),
            ));
        }
        for comment in &parsed.program.comments {
            leaves.push(leaf(
                Kind::Token(if comment.is_line() {
                    Token::Line
                } else {
                    Token::Block
                }),
                Offset(comment.span.start),
                Offset(comment.span.end),
            ));
        }
        merge_lexemes(&mut root, leaves, Offset(readable), Offset(length));
        emit(&mut root, builder);
        for diagnostic in parsed.diagnostics.iter().chain(checked.diagnostics.iter()) {
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
