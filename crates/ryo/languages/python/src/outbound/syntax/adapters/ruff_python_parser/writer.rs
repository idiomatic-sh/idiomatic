//! Serialize native and embedded AST occurrences through one source-order writer.
use super::constants::{ELSE_WITHOUT_EXCEPT, ELSE_WITHOUT_EXCEPT_CODE};
use super::future::{else_without_except, unknown_futures};
use super::lexical::{decoded_of, opens};
use super::schemas::{concrete_node_kind, token_kind};
use super::types::{
    AnnotationCandidate, AnnotationContext, Decoded, Depth, FutureAnnotations, ParserRefusal, Said,
    ScalarBuffer, Strings, TokenIndex, Writer,
};
use crate::domain::extras::types::Extra;
use crate::domain::syntax::constants::{
    ANNOTATION_CONTEXT, ANNOTATION_SEED, CANDIDATE_SEED, MODULE_FUTURE_ANNOTATIONS,
    NORMALIZED_IDENTIFIER,
};
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use ruff_python_ast::token::Tokens;
use ruff_python_ast::visitor::source_order::{
    SourceOrderVisitor, TraversalSignal, walk_interpolated_string_element,
};
use ruff_python_ast::{AnyNodeRef, InterpolatedStringElement};
use ruff_text_size::Ranged as _;
use ryo::languages::{self, DiagnosticMetadata, DiagnosticPhase, Offset, TreeImpact};

/// Visits an element of an f-string or a t-string as the library does, but with an interpolation's format spec entered as the node
/// it is: the library's own walk visits the spec's parts and never the spec, which the standard has as the `JoinedStr` of the
/// interpolation's `format_spec`.
pub(super) fn visit_element<'a, V>(visitor: &mut V, element: &'a InterpolatedStringElement)
where
    V: SourceOrderVisitor<'a>,
{
    let InterpolatedStringElement::Interpolation(interpolation) = element else {
        walk_interpolated_string_element(visitor, element);
        return;
    };
    let node = AnyNodeRef::from(element);
    if visitor.enter_node(node).is_traverse() {
        visitor.visit_expr(&interpolation.expression);
        if let Some(spec) = &interpolation.format_spec {
            let spec_node = AnyNodeRef::from(spec.as_ref());
            if visitor.enter_node(spec_node).is_traverse() {
                for part in &spec.elements {
                    visitor.visit_interpolated_string_element(part);
                }
            }
            visitor.leave_node(spec_node);
        }
    }
    visitor.leave_node(node);
}

/// Writes the text between two tokens that the lexer gives no token for as one `whitespace` token: the gap tokenizer of the tree, so
/// that the tokens read back the whole file.
fn whitespace(builder: &mut languages::TableBuilder<'_>, from: Offset, to: Offset) {
    languages::token(builder, kind_bits(Kind::Extra(Extra::Whitespace)), from, to);
}

impl<'b, 'w, 't> Writer<'b, 'w, 't> {
    /// Start one native or embedded source-order traversal.
    pub(super) fn new(
        builder: &'b mut languages::TableBuilder<'w>,
        tokens: &'t Tokens,
        source: languages::NameText<'t>,
        start: Offset,
        length: Offset,
        context: AnnotationContext,
    ) -> Self {
        Self {
            future_annotations: FutureAnnotations(false),
            annotations: Vec::new(),
            annotation: context,
            start,
            embedding: context,
            tree: languages::lossless(builder, start),
            // A parsed expression's outer parentheses and leading trivia are outside
            // its AST span. They must not be written before its root is opened.
            tokens: if context == AnnotationContext::Annotation {
                tokens.in_range(ruff_text_size::TextRange::new(
                    ruff_text_size::TextSize::new(start.0),
                    ruff_text_size::TextSize::new(length.0),
                ))
            } else {
                tokens
            },
            next: TokenIndex(0),
            depth: Depth(0),
            ends: Vec::new(),
            strings: Vec::new(),
            refused: Vec::new(),
            parsed_tokens: tokens,
            length,
            source,
            scalar: ScalarBuffer::default(),
        }
    }

    /// Where the next token not written starts; the end of the file when every token is written.
    fn next_start(&self) -> Offset {
        self.tokens
            .get(self.next.0)
            .map_or(self.length, |token| Offset(token.start().to_u32()))
    }

    /// Writes the white space between the last token written and `to`, as far as the next token.
    fn fill(&mut self, to: Offset) {
        let to = to.min(self.next_start());
        languages::fill_to(&mut self.tree, to, &mut whitespace);
    }

    /// Writes the next token, after the white space before it; with the text the library decoded for it when it is a string's.
    fn write_next(&mut self, decoded: Option<Decoded<'_>>) {
        let Some(token) = self.tokens.get(self.next.0) else {
            return;
        };
        self.next.0 += 1;
        let (start, end) = (Offset(token.start().to_u32()), Offset(token.end().to_u32()));
        self.fill(start);
        let kind = token_kind(token.kind());
        // Only a string's tokens are written with the text the library decoded; any other token inside the node is written as itself.
        match (kind, decoded) {
            (
                Kind::Token(Token::String | Token::FstringMiddle | Token::TstringMiddle),
                Some(text),
            ) => languages::lossless_decoded_token(
                &mut self.tree,
                kind_bits(kind),
                start,
                end,
                languages::NameText(text.0),
                &mut whitespace,
            ),
            (Kind::Node(_) | Kind::Token(_) | Kind::Extra(_), _) => {
                languages::lossless_token(
                    &mut self.tree,
                    kind_bits(kind),
                    start,
                    end,
                    &mut whitespace,
                );
            }
        }
    }

    /// Writes every token that ends at or before `at`: what comes before a node that starts there. A token of no length where the
    /// innermost open node ends is outside that node, so it waits until the node is closed.
    fn write_before(&mut self, at: Offset) {
        let edge = self.ends.last().copied();
        while self.tokens.get(self.next.0).is_some_and(|token| {
            token.end().to_u32() <= at.0
                && !(token.range().is_empty() && Some(Offset(token.start().to_u32())) == edge)
        }) {
            self.write_next(None);
        }
    }

    /// Writes every token that starts before `at` and ends at or before it: what is inside a node that ends there.
    fn write_within(&mut self, at: Offset, decoded: Option<Decoded<'_>>) {
        while self
            .tokens
            .get(self.next.0)
            .is_some_and(|token| token.start().to_u32() < at.0 && token.end().to_u32() <= at.0)
        {
            self.write_next(decoded);
        }
    }
}

impl<'a> SourceOrderVisitor<'a> for Writer<'_, '_, 'a> {
    fn visit_expr(&mut self, expr: &'a ruff_python_ast::Expr) {
        let previous = self.annotation;
        if matches!(expr, ruff_python_ast::Expr::Call(_)) && previous == AnnotationContext::Runtime
        {
            self.annotation = AnnotationContext::Candidate;
        }
        ruff_python_ast::visitor::source_order::walk_expr(self, expr);
        self.annotation = previous;
    }

    fn visit_stmt(&mut self, stmt: &'a ruff_python_ast::Stmt) {
        let previous = self.annotation;
        if matches!(stmt, ruff_python_ast::Stmt::AnnAssign(_))
            && previous == AnnotationContext::Runtime
        {
            self.annotation = AnnotationContext::Candidate;
        }
        ruff_python_ast::visitor::source_order::walk_stmt(self, stmt);
        self.annotation = previous;
    }
    fn visit_annotation(&mut self, expr: &'a ruff_python_ast::Expr) {
        let previous = self.annotation;
        self.annotation = AnnotationContext::Annotation;
        ruff_python_ast::visitor::source_order::walk_annotation(self, expr);
        self.annotation = previous;
    }
    fn visit_interpolated_string_element(
        &mut self,
        interpolated_string_element: &'a InterpolatedStringElement,
    ) {
        visit_element(self, interpolated_string_element);
    }

    fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
        let start = if self.depth.0 == 0 {
            self.start
        } else {
            Offset(node.start().to_u32())
        };
        self.write_before(start);
        self.fill(start);
        if self.annotation != AnnotationContext::Runtime
            && let AnyNodeRef::ExprStringLiteral(literal) = node
        {
            self.annotations.push(AnnotationCandidate {
                host: languages::builder_position(self.tree.builder),
                literal,
            });
        }
        let strings = self.strings.last().copied().unwrap_or(Strings::Formatted);
        languages::enter_node(
            self.tree.builder,
            kind_bits(concrete_node_kind(node, strings)),
            start,
        );
        if matches!(node, AnyNodeRef::ExprStringLiteral(_))
            && self.annotation != AnnotationContext::Runtime
        {
            languages::node_property(
                self.tree.builder,
                ANNOTATION_CONTEXT,
                languages::NameText(if self.annotation == AnnotationContext::Annotation {
                    ANNOTATION_SEED
                } else {
                    CANDIDATE_SEED
                }),
            );
        }
        if matches!(node, AnyNodeRef::ModModule(_)) {
            languages::node_property(
                self.tree.builder,
                MODULE_FUTURE_ANNOTATIONS,
                languages::NameText(if self.future_annotations.0 {
                    crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS_ENABLED
                } else {
                    crate::domain::syntax::constants::MODULE_FUTURE_ANNOTATIONS_DISABLED
                }),
            );
        }
        scalar_properties(node, self.tree.builder, &mut self.scalar);
        let canonical = if let AnyNodeRef::Identifier(identifier) = node {
            Some(identifier.as_str())
        } else if let AnyNodeRef::ExprName(name) = node {
            Some(name.id.as_str())
        } else {
            None
        };
        if let Some(canonical) = canonical {
            let raw = self
                .source
                .0
                .get(usize::from(node.start())..usize::from(node.end()))
                .unwrap_or_default();
            if raw != canonical || self.embedding == AnnotationContext::Annotation {
                languages::node_property(
                    self.tree.builder,
                    NORMALIZED_IDENTIFIER,
                    languages::NameText(canonical),
                );
                languages::node_property(
                    self.tree.builder,
                    languages::PropertyId(0),
                    languages::NameText(canonical.split('.').next().unwrap_or(canonical)),
                );
            }
        }
        if let Some(opened) = opens(node.kind()) {
            self.strings.push(opened);
        }
        if let AnyNodeRef::StmtTry(statement) = node
            && let Some(at) = else_without_except(statement, self.parsed_tokens)
        {
            self.refused.push(ParserRefusal {
                at,
                said: Said(ELSE_WITHOUT_EXCEPT.to_owned()),
                metadata: DiagnosticMetadata { phase: DiagnosticPhase::Parse, impact: TreeImpact::Incomplete, code: ELSE_WITHOUT_EXCEPT_CODE },
            });
        }
        if let AnyNodeRef::StmtImportFrom(statement) = node {
            self.refused.extend(unknown_futures(statement));
        }
        self.depth.0 += 1;
        self.ends.push(if self.depth.0 == 1 {
            self.length
        } else {
            Offset(node.end().to_u32())
        });
        TraversalSignal::Traverse
    }

    fn leave_node(&mut self, node: AnyNodeRef<'a>) {
        self.depth.0 = self.depth.0.saturating_sub(1);
        let _: Option<Offset> = self.ends.pop();
        if opens(node.kind()).is_some() {
            let _: Option<Strings> = self.strings.pop();
        }
        let end = if self.depth.0 == 0 {
            while self.next.0 < self.tokens.len() {
                self.write_next(None);
            }
            self.length
        } else {
            let end = Offset(node.end().to_u32());
            self.write_within(end, decoded_of(node));
            end
        };
        self.fill(end);
        languages::leave_node(self.tree.builder, end);
    }
}

/// Preserve scalar AST payloads without exposing parser types to semantic readers.
/// The parser has already decoded literals; numeric variants remain distinct from runtime coercions.
fn scalar_properties(
    node: AnyNodeRef<'_>,
    builder: &mut languages::TableBuilder<'_>,
    scratch: &mut ScalarBuffer,
) {
    use crate::domain::syntax::constants::{
        BOOLEAN_SCALAR, BYTES_SCALAR, COMPLEX_SCALAR, ELLIPSIS_SCALAR, FLOAT_SCALAR,
        INTEGER_SCALAR, INTERPOLATION_CONVERSION, INTERPOLATION_DEBUG, NONE_SCALAR,
        SCALAR_IMAGINARY, SCALAR_KIND, SCALAR_VALUE,
    };
    use ruff_python_ast::Number;
    use ryo::languages::{NameText, node_property};
    if let AnyNodeRef::ExprNumberLiteral(literal) = node {
        match &literal.value {
            Number::Int(value) => {
                node_property(builder, SCALAR_KIND, NameText(INTEGER_SCALAR));
                scalar_property(builder, scratch, SCALAR_VALUE, value);
            }
            Number::Float(value) => {
                node_property(builder, SCALAR_KIND, NameText(FLOAT_SCALAR));
                scalar_property(builder, scratch, SCALAR_VALUE, value.to_bits());
            }
            Number::Complex { real, imag } => {
                node_property(builder, SCALAR_KIND, NameText(COMPLEX_SCALAR));
                scalar_property(builder, scratch, SCALAR_VALUE, real.to_bits());
                scalar_property(builder, scratch, SCALAR_IMAGINARY, imag.to_bits());
            }
        }
    } else if let AnyNodeRef::ExprBooleanLiteral(literal) = node {
        node_property(builder, SCALAR_KIND, NameText(BOOLEAN_SCALAR));
        scalar_property(builder, scratch, SCALAR_VALUE, literal.value);
    } else if matches!(node, AnyNodeRef::ExprNoneLiteral(_)) {
        node_property(builder, SCALAR_KIND, NameText(NONE_SCALAR));
    } else if matches!(node, AnyNodeRef::ExprEllipsisLiteral(_)) {
        node_property(builder, SCALAR_KIND, NameText(ELLIPSIS_SCALAR));
    } else if let AnyNodeRef::ExprBytesLiteral(literal) = node {
        node_property(builder, SCALAR_KIND, NameText(BYTES_SCALAR));
        scalar_property(
            builder,
            scratch,
            SCALAR_VALUE,
            super::types::ScalarBytes(&literal.value),
        );
    } else if let AnyNodeRef::InterpolatedElement(interpolation) = node {
        if let Some(debug) = &interpolation.debug_text {
            scalar_property(
                builder,
                scratch,
                INTERPOLATION_DEBUG,
                super::types::DebugSpelling(debug.as_str()),
            );
        }
        if let Some(conversion) = interpolation.conversion.to_char() {
            scalar_property(builder, scratch, INTERPOLATION_CONVERSION, conversion);
        }
    } else {
        // Other syntax nodes carry no scalar parser payload.
    }
}

/// Format into a reusable `String` before adding a scalar property; formatting a `String` is infallible.
fn scalar_property(
    builder: &mut languages::TableBuilder<'_>,
    scratch: &mut ScalarBuffer,
    property: languages::PropertyId,
    value: impl std::fmt::Display,
) {
    use std::fmt::Write as _;
    scratch.0.clear();
    // `String` implements `fmt::Write` infallibly; assert that contract while consuming the result.
    let result = write!(scratch.0, "{value}");
    debug_assert!(result.is_ok(), "formatting into String is infallible");
    languages::node_property(builder, property, languages::NameText(&scratch.0));
}
