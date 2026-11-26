//! Read TypeScript with Oxc and project its typed AST into TypeScript-ESTree nodes.

use super::constants::{IMPORT_META, IMPORT_OPTIONS_KEY, NEW_TARGET, NOT_UTF8};
use super::flow::service::drafted_flow;
use super::output::{Numbered, Output};
use super::patterns::service::{carried, constructed};
use super::projection::service::{annotate, attach, expression_body, gap, insert, jsx_names, leaf, meta_words, node_properties, normalized, parameter};
use super::schemas::{node_kind, source_type_of, token_kind};
use super::types::{ChildCursor, Decoded, DecodedText, FlowDraft, NodePositions, NodeProperty, Piece, Text, VisitorStack};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::constants::{LOWERED_FLAG, RAISED_FLAG, TYPESCRIPT_SYNTAX_FLAG};
use crate::domain::syntax::types::{Dialect, Kind};
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::{SourceText, Wants, Walked};
use oxc_allocator::Allocator;
use oxc_ast::{AstKind, ast::{ArrowFunctionExpression, FormalParameter, FormalParameterRest, ImportMeta, JSXAttribute, JSXClosingElement, JSXMemberExpression, JSXNamespacedName, JSXOpeningElement, NewTarget, TSGlobalDeclaration, VariableDeclarator}};
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
                value: value.as_str(),
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
                value: value.name.as_str(),
                token_kind: Token::PrivateIdentifierToken,
            });
        }
        // A template with no substitution is a string: its one quasi's cooked value, which a tagged template's tag reads raw instead.
        if let AstKind::TemplateLiteral(template) = kind
            && template.expressions.is_empty()
            && !self
                .open
                .last()
                .is_some_and(|piece| piece.kind == Kind::Node(Node::TaggedTemplateExpression))
            && let [quasi] = template.quasis.as_slice()
            && let Some(cooked) = &quasi.value.cooked
        {
            self.decoded.push(Decoded {
                span: template.span,
                value: cooked.as_str(),
                token_kind: Token::Template,
            });
        }
        if let AstKind::StringLiteral(literal) = kind {
            self.decoded.push(Decoded {
                span: literal.span,
                value: literal.value.as_str(),
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
            properties: node_properties(&self.scoping, kind),
            carries,
            node: Some(kind.node_id()),
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

    fn walk(
        &self,
        text: SourceText<'_>,
        dialect: Dialect,
        builder: &mut languages::NodeTableBuilder<'_>,
        wants: Wants,
    ) -> Walked {
        let mut out = Numbered::new(builder, wants);
        project(text, dialect, &mut out);
        Walked { flow: out.flow, patterns: out.patterns, decoded: out.decoded }
    }
}

/// Reads the text with Oxc and projects it into the standard tree, which `out` writes.
#[expect(
    clippy::too_many_lines,
    reason = "The parser result, tokens and diagnostics share one borrowed parse lifetime; this operation keeps their projection at that boundary"
)]
fn project(text: SourceText<'_>, dialect: Dialect, out: &mut dyn Output) {
    {
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
        // References and symbol cells are used by the syntax projection. Only
        // the four flow relations publish the graph and its node memberships.
        let needs_flow = out.needs_flow();
        let semantic = oxc_semantic::SemanticBuilder::new()
            .with_check_syntax_error(true)
            .with_build_nodes(true)
            .with_cfg(needs_flow)
            .build(&parsed.program);
        let mut flow = needs_flow.then(|| {
            let mut draft = FlowDraft::default();
            drafted_flow(&semantic.semantic, &mut draft);
            (draft, NodePositions(vec![None; semantic.semantic.nodes().len()]))
        });
        let mut projection = VisitorStack {
            scoping: semantic.semantic.into_scoping(),
            ..VisitorStack::default()
        };
        projection.visit_program(&parsed.program);
        projection
            .decoded
            .sort_by_key(|decoded| (decoded.span.start, decoded.span.end));
        let length = u32::try_from(text.0.len()).unwrap_or(u32::MAX);
        let mut root = projection.roots.remove(0);
        root.span = Span::new(0, length);
        // Whether the file is TypeScript, as Oxc's source type says: the fact oxlint's JavaScript-only rules test.
        root.properties.push(NodeProperty {
            id: TYPESCRIPT_SYNTAX_FLAG,
            value: DecodedText(
                if source_type.is_typescript() {
                    RAISED_FLAG
                } else {
                    LOWERED_FLAG
                }
                .to_owned(),
            ),
        });
        let mut leaves = Vec::new();
        for token in &parsed.tokens {
            if token.start() < token.end() {
                let span = Span::new(token.start(), token.end());
                let mapped = match token_kind(token.kind()) {
                    Ok(token) => Kind::Token(token),
                    Err(unrecognized) => {
                        out.diagnose(
                            &format!("unrecognized parser token: {unrecognized:?}"),
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
                    piece.text = Text::Decoded(DecodedText(decoded.value.to_owned()));
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
        let positions = flow
            .as_mut()
            .map(|(_, positions)| positions.0.as_mut_slice())
            .unwrap_or_default();
        out.tree(&mut root, &mut projection.carriers, positions, source);
        if let Some((flow, positions)) = flow {
            out.flow(&flow, &positions);
        }
        out.patterns(
            &allocator,
            languages::NameText(source),
            &projection.carriers,
            &projection.scoping,
        );
        // The parser accepts a file when neither it nor the semantic pass's syntax check reported an error and every byte was
        // readable: the files oxlint lints, where it reports only the error for any other.
        out.accept(
            if parsed.diagnostics.is_empty()
                && semantic.diagnostics.is_empty()
                && readable == length
            {
                languages::SyntaxAcceptance::Accepted
            } else {
                languages::SyntaxAcceptance::Rejected
            },
        );
        for diagnostic in parsed.diagnostics.iter().chain(semantic.diagnostics.iter()) {
            let span = diagnostic
                .labels
                .as_slice()
                .first()
                .map_or(Span::new(0, readable), |label| {
                    Span::new(label.offset(), label.offset().saturating_add(label.len()))
                });
            out.diagnose(&diagnostic.to_string(), Offset(span.start), Offset(span.end));
        }
        if readable < length {
            out.diagnose(NOT_UTF8, Offset(readable), Offset(length));
        }
    }
}
