//! Read TypeScript with Oxc and project its typed AST into TypeScript-ESTree nodes.

use super::constants::{IMPORT_META, IMPORT_OPTIONS_KEY, NEW_TARGET, NOT_UTF8};
use super::patterns::service::{carried, constructed, written_patterns};
use super::projection::service::{
    annotate, attach, expression_body, gap, insert, jsx_names, leaf, meta_words, normalized,
    parameter, source_type_of, write,
};
use super::schemas::{node_kind, token_kind};
use super::types::{ChildCursor, Decoded, DecodedText, Piece, Text, VisitorStack};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::{Dialect, Kind};
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
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
use oxc_span::{GetSpan as _, Span};
use ryo::languages::{self, Offset};

/// The Oxc syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxOxcParserAdapter;

impl<'a> Visit<'a> for VisitorStack<'a> {
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
        constructed(self, kind);
        let carries = carried(self, kind);
        self.open.push(Piece {
            kind: node_kind(kind).map_or(Kind::Extra(Extra::Error), Kind::Node),
            span: kind.span(),
            children: Vec::new(),
            text: Text::Source,
            cursor: ChildCursor(0),
            carries,
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
        write(&mut root, &mut projection.carriers, builder);
        let semantic = oxc_semantic::SemanticBuilder::new()
            .with_check_syntax_error(true)
            .build(&parsed.program);
        written_patterns(
            &allocator,
            languages::NameText(source),
            &projection.carriers,
            &semantic.semantic,
            builder,
        );
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
