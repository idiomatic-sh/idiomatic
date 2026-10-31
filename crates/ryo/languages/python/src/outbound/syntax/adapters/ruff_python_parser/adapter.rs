// ignore: idiomatic.ryo/language/no-wildcard-into-domain-enum - the one `_ =>` arm of this adapter is in `decoded_of`, a match over the library's hundred-odd node kinds whose answer is the text the library decoded, `Option<Decoded>`, not a domain enum: only two kinds carry such text, so a kind the library gains has none until it is named, and naming each of the others would be a hundred arms that all say None
//! The syntax port, implemented with `ruff_python_parser`, the parser ruff and ty run, for the latest Python it reads: its tree of
//! nodes and its stream of tokens are written into the element table together, in the order of the file. The library's tree is an
//! abstract one: punctuation, keywords and comments are only in its tokens. Each node is opened where it starts, after every token
//! that ends before it, and closed where it ends, after every token that starts inside it; the text between two tokens that the lexer
//! gives no token for (spaces, a line continuation) is a `whitespace` token, so the tokens of the table spell the file. A string's
//! token carries the text the library decoded for it.
//!
//! What the library says of the file is its diagnostics: the errors of its parse, the syntax the version it reads does not have, and
//! the bytes that are not UTF-8. The library's names stop here; what leaves is the language's `Kind`, the standards' names.

use std::borrow::Cow;

use ruff_python_ast::token::{Token as RuffToken, Tokens};
use ruff_python_ast::visitor::source_order::{
    SourceOrderVisitor, TraversalSignal, walk_interpolated_string_element,
};
use ruff_python_ast::{AnyNodeRef, InterpolatedStringElement, PythonVersion};
use ruff_python_parser::{Mode, ParseOptions, parse_unchecked};
use ruff_text_size::Ranged as _;
use ryo::languages::{self, Offset};

use crate::domain::extras::types::Extra;
use crate::domain::syntax::constants::NORMALIZED_IDENTIFIER;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

use super::constants::{ELSE_WITHOUT_EXCEPT, NOT_UTF8, UNREADABLE};
use super::schemas::{else_without_except, node_kind, opens, token_kind, unknown_futures};
use super::types::{ByteIndex, Decoded, Depth, ParserRefusal, Readable, Said, Strings, TokenIndex};

/// The `ruff_python_parser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxRuffPythonParserAdapter;

/// The offset of a byte index of the text.
fn offset(index: ByteIndex) -> Offset {
    Offset(u32::try_from(index.0).unwrap_or(u32::MAX))
}

/// The text the library reads for `bytes`: the bytes themselves when they are UTF-8, or else each byte that is not part of a
/// character replaced by one the lexer cannot read, so every offset is the file's.
pub(super) fn readable(bytes: SourceText<'_>) -> Readable<'_> {
    if let Ok(text) = std::str::from_utf8(bytes.0) {
        return Readable(Cow::Borrowed(text));
    }
    let mut text = String::with_capacity(bytes.0.len());
    for chunk in bytes.0.utf8_chunks() {
        text.push_str(chunk.valid());
        text.extend(chunk.invalid().iter().map(|_| UNREADABLE));
    }
    Readable(Cow::Owned(text))
}

/// The options the library parses with: a module, in the latest Python it reads.
pub(super) fn options() -> ParseOptions {
    ParseOptions::from(Mode::Module).with_target_version(PythonVersion::latest())
}

/// The text the library decoded for the tokens of a node: a string's value, or the literal part of an f-string's or a t-string's. Only
/// these two kinds of the library's nodes carry it; every other kind carries none.
#[expect(
    clippy::wildcard_enum_match_arm,
    reason = "only the nodes of a string's text carry what the library decoded; the other ninety-odd kinds carry nothing, and a kind the library gains carries nothing until it is named here"
)]
fn decoded_of(node: AnyNodeRef<'_>) -> Option<Decoded<'_>> {
    match node {
        AnyNodeRef::StringLiteral(literal) => Some(Decoded(&literal.value)),
        AnyNodeRef::InterpolatedStringLiteralElement(element) => Some(Decoded(&element.value)),
        _ => None,
    }
}

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

/// The writer of the table: it walks the library's tree in the order of the file and writes its tokens between the nodes.
struct Writer<'b, 'w, 't> {
    /// The tree, and where its last token ends.
    tree: languages::Lossless<'b, 'w>,
    /// The library's tokens, in the order of the file.
    tokens: &'t [RuffToken],
    /// The first token not written yet.
    next: TokenIndex,
    /// How many nodes are open.
    depth: Depth,
    /// Where each open node ends, the innermost last.
    ends: Vec<Offset>,
    /// The strings open, the innermost last: what an interpolation is named by.
    strings: Vec<Strings>,
    /// The library's tokens, to find a keyword in.
    parsed_tokens: &'t Tokens,
    /// Where `CPython` refuses what the library reads: what the adapter reports after the library's own errors.
    refused: Vec<ParserRefusal>,
    /// How long the file is.
    length: Offset,
    /// Source spelling retained only to distinguish normalized identifiers.
    source: languages::NameText<'t>,
}

impl Writer<'_, '_, '_> {
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

impl<'a> SourceOrderVisitor<'a> for Writer<'_, '_, '_> {
    fn visit_interpolated_string_element(
        &mut self,
        interpolated_string_element: &'a InterpolatedStringElement,
    ) {
        visit_element(self, interpolated_string_element);
    }

    fn enter_node(&mut self, node: AnyNodeRef<'a>) -> TraversalSignal {
        let start = if self.depth.0 == 0 {
            Offset(0)
        } else {
            Offset(node.start().to_u32())
        };
        self.write_before(start);
        self.fill(start);
        let strings = self.strings.last().copied().unwrap_or(Strings::Formatted);
        languages::enter_node(
            self.tree.builder,
            kind_bits(node_kind(node.kind(), strings)),
            start,
        );
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
            if raw != canonical {
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

impl SyntaxPort for SyntaxRuffPythonParserAdapter {
    fn read(&self, text: SourceText<'_>, builder: &mut languages::TableBuilder<'_>) {
        let Readable(source) = readable(text);
        let parsed = parse_unchecked(&source, options());
        let length = offset(ByteIndex(text.0.len()));
        let mut writer = Writer {
            tree: languages::lossless(builder, Offset(0)),
            tokens: parsed.tokens(),
            next: TokenIndex(0),
            depth: Depth(0),
            ends: Vec::new(),
            strings: Vec::new(),
            refused: Vec::new(),
            parsed_tokens: parsed.tokens(),
            length,
            source: languages::NameText(&source),
        };
        writer.visit_mod(parsed.syntax());
        if source.len() == text.0.len() && matches!(source, Cow::Owned(_)) {
            let mut at = 0_usize;
            for chunk in text.0.utf8_chunks() {
                at += chunk.valid().len();
                let bad = chunk.invalid().len();
                if bad > 0 {
                    languages::diagnose(
                        writer.tree.builder,
                        languages::ReportText(NOT_UTF8),
                        offset(ByteIndex(at)),
                        offset(ByteIndex(at + bad)),
                    );
                }
                at += bad;
            }
        }
        for error in parsed.errors() {
            // The error's own message: the library's `Display` of the whole error adds the byte range the row already holds.
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(&error.error.to_string()),
                Offset(error.location.start().to_u32()),
                Offset(error.location.end().to_u32()),
            );
        }
        for error in parsed.unsupported_syntax_errors() {
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(&error.to_string()),
                Offset(error.range.start().to_u32()),
                Offset(error.range.end().to_u32()),
            );
        }
        let refused = std::mem::take(&mut writer.refused);
        for refusal in refused {
            languages::diagnose(
                writer.tree.builder,
                languages::ReportText(&refusal.said.0),
                Offset(refusal.at.start().to_u32()),
                Offset(refusal.at.end().to_u32()),
            );
        }
    }
}
