//! The writer of the adapter: place, contents, body_of, target and more.

use super::super::constants::{IMPORT, RULE_BODIES, URL};
use super::super::schemas::{decoded_text, error_message, met, pieces, token_kind};
use super::super::types::{
    Body, ByteIndex, Closing, Contents, CssText, Met, Open, Pieces, Recovered, Scratch,
};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::service::mirror;
use cssparser::{ParseError, Parser, RuleBodyParser, StyleSheetParser, Token};
use ryo::languages::SourcePath;
use ryo::languages::{self, Count, NameText, Offset};

/// Where the parser is, in bytes from the start of its text.
pub(crate) fn place(input: &Parser<'_>) -> ByteIndex {
    ByteIndex(input.position().byte_index())
}

/// Whether the value `input` holds has a `{}` block at its top beside other content.
pub(crate) fn contents(input: &mut Parser<'_>) -> Contents {
    let (mut block, mut content) = (false, Count(0));
    while let Ok(token) = input.next() {
        block |= matches!(token, Token::CurlyBracketBlock);
        content.0 += 1;
    }
    if block && content.0 > 1 {
        Contents::HoldsBlock
    } else {
        Contents::Plain
    }
}

/// What the body of the at-rule `name` holds: rules for an at-rule CSS defines so, with or without a vendor prefix (`-webkit-`).
pub(crate) fn body_of(name: NameText<'_>) -> Body {
    let unprefixed = name
        .0
        .strip_prefix('-')
        .and_then(|rest| rest.split_once('-'))
        .map_or(name.0, |(_, rest)| rest);
    if RULE_BODIES
        .iter()
        .any(|each| each.eq_ignore_ascii_case(unprefixed))
    {
        Body::Rules
    } else {
        Body::Components
    }
}

/// The target the token `parser` has just read names, resolved against `location`: of a url-token its url, of a `url(` its string,
/// of an `@import` its string or url; nothing for any other token. The parser is set back where it was.
pub(in super::super) fn target(
    parser: &mut Parser<'_>,
    token: &Token<'_>,
    location: SourcePath<'_>,
) -> Option<languages::ResolvedTarget> {
    let url = if let Token::UnquotedUrl(url) = token {
        Some(url.to_string())
    } else if let Token::Function(name) = token
        && name.eq_ignore_ascii_case(URL)
    {
        let state = parser.state();
        let found = parser.parse_nested_block(|inner| {
            Ok::<_, ParseError<()>>(inner.expect_string()?.to_string())
        });
        parser.reset(&state);
        // a `url(` whose block holds no string names no target: the parser's refusal is the answer that there is none
        let Ok(found) = found else {
            return None;
        };
        Some(found)
    } else if let Token::AtKeyword(name) = token
        && name.eq_ignore_ascii_case(IMPORT)
    {
        let state = parser.state();
        let found = parser.expect_url_or_string().map(|url| url.to_string());
        parser.reset(&state);
        // an `@import` followed by no url or string names no target: the parser's refusal is the answer that there is none
        let Ok(found) = found else {
            return None;
        };
        Some(found)
    } else {
        None
    };
    url.map(|written| languages::resolve_target(location, languages::WrittenTarget(&written)))
}

/// The kind of the token that closes the block `token` opens: its mirror; nothing for a token that opens none.
pub(in super::super) fn closer(token: Kind) -> Option<Kind> {
    match token {
        Kind::Token(lexeme) => mirror(lexeme).map(Kind::Token),
        Kind::Node(_) | Kind::Extra(_) => None,
    }
}

/// The writer of the table: the library's callbacks, and the tokens between them.
pub(in super::super) struct Writer<'b, 'w, 't> {
    /// The builder of the file's element table, which every node and token is written through.
    pub(in super::super) builder: &'b mut languages::TableBuilder<'w>,
    /// The text the parser reads: the file after its byte order mark, up to its first byte that is not UTF-8.
    pub(in super::super) text: CssText<'t>,
    /// Where the file is, from the root of the run: what its urls are resolved against.
    pub(in super::super) location: SourcePath<'t>,
    /// Where `text` starts in the file.
    pub(in super::super) base: ByteIndex,
    /// How far into `text` the tokens are written.
    pub(in super::super) written: ByteIndex,
    /// Where the last component value the parse met ends: where a node that ends at its content ends.
    pub(in super::super) content_end: ByteIndex,
    /// The nodes open, the stylesheet first.
    pub(in super::super) open: Vec<Open>,
    /// The text of a message, kept for the file.
    pub(in super::super) scratch: Scratch,
}

impl Writer<'_, '_, '_> {
    /// The offset in the file of a place of `text`.
    pub(in super::super) fn offset(&self, at: ByteIndex) -> Offset {
        Offset(u32::try_from(self.base.0 + at.0).unwrap_or(u32::MAX))
    }

    /// Writes one token of `kind` from `start` to `end` of `text`, with the text the library decoded from it.
    pub(in super::super) fn put(
        &mut self,
        kind: Kind,
        start: ByteIndex,
        end: ByteIndex,
        decoded: Option<NameText<'_>>,
    ) {
        let (from, to) = (self.offset(start), self.offset(end));
        match decoded {
            Some(text) => languages::decoded_token(self.builder, kind_bits(kind), from, to, text),
            None => languages::token(self.builder, kind_bits(kind), from, to),
        }
    }

    /// Writes every token the library's tokenizer reads of the text `parser` reads, `shift` bytes into `text`, those inside the
    /// blocks it opens too, up to `limit`, and answers where the last one ends. The parser reads on past `limit`, so that a token
    /// that ends there is read as it is in the file (a string a line end breaks is a bad-string-token only with its line end after
    /// it). A token the library reads where the standard reads two delim-tokens is written as the two; a block nested past the
    /// library's limit is written as one token of text the parser could not read.
    pub(in super::super) fn tokens(
        &mut self,
        parser: &mut Parser<'_>,
        shift: ByteIndex,
        limit: ByteIndex,
    ) -> ByteIndex {
        let mut last = ByteIndex(shift.0 + place(parser).0);
        loop {
            let start = ByteIndex(shift.0 + place(parser).0);
            if start >= limit {
                return last;
            }
            // The token is the library's, its text shared with the parser's: a clone is a count, not a copy.
            let Ok(token) = parser.next_including_whitespace_and_comments().cloned() else {
                return last;
            };
            let end = ByteIndex(shift.0 + place(parser).0);
            let kind = token_kind(&token);
            let pointed = target(parser, &token, self.location);
            match (pieces(&token), pointed) {
                (Pieces::TwoDelims, _) => {
                    let middle = ByteIndex(start.0 + 1);
                    self.put(kind, start, middle, None);
                    self.put(kind, middle, end, None);
                }
                (Pieces::Whole, Some(path)) => self.put(kind, start, end, Some(NameText(&path.0))),
                (Pieces::Whole, None) => self.put(kind, start, end, decoded_text(&token)),
            }
            last = end;
            if let Some(closing) = closer(kind) {
                // The library refuses a block the closure leaves unread, so where the block's tokens end is kept beside its answer.
                let mut inside = None;
                let nested: Result<(), ParseError<()>> = parser.parse_nested_block(|inner| {
                    inside = Some(self.tokens(inner, shift, limit));
                    Ok(())
                });
                if let Some(reached) = inside
                    && reached >= limit
                {
                    return reached;
                }
                let after = ByteIndex(shift.0 + place(parser).0);
                match (nested, inside) {
                    (Ok(()), Some(inside)) if after > inside => {
                        self.put(closing, inside, after, None);
                    }
                    (Err(_), None) if after > end => {
                        self.put(Kind::Extra(Extra::Error), end, after, None);
                    }
                    (Ok(()) | Err(_), Some(_) | None) => {}
                }
                last = after;
            }
        }
    }

    /// Writes the tokens of `text` up to `to`.
    pub(in super::super) fn advance(&mut self, to: ByteIndex) {
        if to <= self.written {
            return;
        }
        let from = self.written;
        let text = self.text;
        let mut parser = Parser::new(text.0.get(from.0..).unwrap_or_default());
        // A token that starts before `to` is written whole, so what is written reaches where the last one ends.
        let reached = self.tokens(&mut parser, from, to);
        self.written = reached.max(to);
    }

    /// Where the at-keyword that ends at `at` starts: the last token before it.
    pub(in super::super) fn keyword_start(&self, at: ByteIndex) -> ByteIndex {
        let from = self.written.min(at);
        let mut parser = Parser::new(self.text.0.get(from.0..at.0).unwrap_or_default());
        let mut start = from;
        loop {
            let before = ByteIndex(from.0 + place(&parser).0);
            if parser.next_including_whitespace_and_comments().is_err() {
                return start;
            }
            start = before;
        }
    }

    /// Opens a node at `at`, after writing the tokens before it.
    pub(in super::super) fn enter(&mut self, node: Node, at: ByteIndex) {
        self.advance(at);
        languages::enter_node(self.builder, kind_bits(Kind::Node(node)), self.offset(at));
        self.open.push(Open {
            node,
            closing: Closing::AtContent,
        });
    }

    /// Closes the innermost node at `at`, after writing the tokens before it.
    pub(in super::super) fn leave(&mut self, at: ByteIndex) {
        let at = at.max(self.written);
        self.advance(at);
        languages::leave_node(self.builder, self.offset(at));
        let _: Option<Open> = self.open.pop();
    }

    /// Closes the nodes an item of a list left open, down to `mark`: a rule whose body was opened where the parser is, after its
    /// `}`, and any other node at its content.
    pub(in super::super) fn close_to(&mut self, mark: Count, at: ByteIndex) {
        while self.open.len() > mark.0 {
            let end = match self.open.last().map(|each| each.closing) {
                Some(Closing::AtPlace) => at,
                Some(Closing::AtContent) | None => self.content_end,
            };
            self.leave(end);
            self.content_end = end;
        }
    }

    /// Records what the library says of the rule or declaration `slice` of `text` it recovered from.
    pub(in super::super) fn recovered(&mut self, error: &ParseError<()>, slice: Recovered<'_>) {
        let start = slice
            .0
            .as_ptr()
            .addr()
            .saturating_sub(self.text.0.as_ptr().addr());
        error_message(&error.kind, &mut self.scratch);
        let (from, to) = (
            self.offset(ByteIndex(start)),
            self.offset(ByteIndex(start + slice.0.len())),
        );
        languages::diagnose(
            self.builder,
            languages::ReportText(&self.scratch.0),
            from,
            to,
        );
    }

    /// Records what the language says of the token from `start` to `end`, which the library calls a parse error.
    pub(in super::super) fn broken(
        &mut self,
        message: languages::ReportText<'static>,
        start: ByteIndex,
        end: ByteIndex,
    ) {
        let (from, to) = (self.offset(start), self.offset(end));
        languages::diagnose(self.builder, message, from, to);
    }

    /// Marks the rule open at the top as one whose body the parser has just entered: it ends after its `}`.
    pub(in super::super) fn open_body(&mut self) {
        if let Some(rule) = self.open.last_mut() {
            rule.closing = Closing::AtPlace;
        }
    }

    /// Walks the component values `input` gives: it opens the node of each function and simple block, and records what the library
    /// calls a parse error.
    pub(in super::super) fn components(&mut self, input: &mut Parser<'_>) {
        loop {
            let start = place(input);
            let Ok(token) = input.next_including_whitespace_and_comments() else {
                break;
            };
            let found = met(token);
            let end = place(input);
            match found {
                Met::Trivia => {}
                Met::Opens(node) => {
                    self.enter(node, start);
                    let nested: Result<(), ParseError<()>> = input.parse_nested_block(|inner| {
                        self.components(inner);
                        Ok(())
                    });
                    let after = place(input);
                    if let Err(error) = nested {
                        error_message(&error.kind, &mut self.scratch);
                        let (from, to) = (self.offset(start), self.offset(after));
                        languages::diagnose(
                            self.builder,
                            languages::ReportText(&self.scratch.0),
                            from,
                            to,
                        );
                    }
                    self.leave(after);
                    self.content_end = after;
                }
                Met::Broken(message) => {
                    self.broken(message, start, end);
                    self.content_end = end;
                }
                Met::Content => self.content_end = end,
            }
        }
    }

    /// Reads the rules and declarations of a body, each closed as the library finishes it.
    pub(in super::super) fn rules(&mut self, input: &mut Parser<'_>) {
        let mut body = RuleBodyParser::<_, (), ()>::new(input, self);
        loop {
            let mark = Count(body.parser.open.len());
            let Some(item) = body.next() else {
                break;
            };
            if let Err((error, slice, _)) = item {
                body.parser.recovered(&error, Recovered(slice));
            }
            let at = place(body.input);
            body.parser.close_to(mark, at);
        }
    }

    /// Reads the rules of the stylesheet, each closed as the library finishes it.
    pub(in super::super) fn stylesheet(&mut self, input: &mut Parser<'_>) {
        let mut sheet = StyleSheetParser::new(input, self);
        loop {
            let mark = Count(sheet.parser.open.len());
            let Some(item) = sheet.next() else {
                break;
            };
            if let Err((error, slice, _)) = item {
                sheet.parser.recovered(&error, Recovered(slice));
            }
            let at = place(sheet.input);
            sheet.parser.close_to(mark, at);
        }
    }
}
