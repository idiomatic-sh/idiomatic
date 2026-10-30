// ignore: idiomatic.arch/file-length - the adapter is the receiver of the library's rule parsers, one method for each callback of its traits, and the tree it writes from them; splitting it would make a second file no adapter template places
//! The syntax port, implemented with `cssparser`, Servo's CSS Syntax Level 3 parser. The library gives no tree: a tokenizer, nested
//! blocks, and rule and declaration parsers that call back for each rule's prelude and body and each declaration's value. The
//! adapter is those callbacks: each opens and closes the node of the standard's parsing results it is given (a rule, a declaration,
//! a function, a simple block) at the place the library is at, and the tokens between are written as the library's tokenizer reads
//! them, every byte in one, so the tree's tokens spell the file. A node ends at its last component value, or after the `}` of a
//! rule's body: white space and comments after it belong to its parent.
//!
//! A rule's body holds rules and declarations when the rule is a style rule or an at-rule CSS defines so (`RULE_BODIES`), and
//! component values otherwise. A declaration whose value holds a `{}` block beside other content is read again as a nested rule, as
//! CSS Syntax says. What the library says of the stylesheet is its errors: each rule and declaration its parsers recover from, each
//! token its tokenizer calls a parse error, and each block nested past its limit. The library's names stop here; what leaves is the
//! language's own `Kind`.

use cssparser::{
    AtRuleParser, BasicParseErrorKind, CowRcStr, DeclarationParser, ParseError, Parser,
    ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, Token,
};
use ryo::languages::SourcePath;
use ryo::languages::{self, Count, NameText, Offset};

use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::service::mirror;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;

use super::constants::{
    BYTE_ORDER_MARK, CHARSET, CUSTOM_PROPERTY, IMPORT, NOT_UTF8, RULE_BODIES, URL,
};
use super::schemas::{decoded_text, error_message, met, pieces, token_kind};
use super::types::{
    Body, ByteIndex, Closing, Contents, CssText, Met, Open, Pieces, Recovered, Scratch,
};

/// The `cssparser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxCssparserAdapter;

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

/// Where the parser is, in bytes from the start of its text.
pub(crate) fn place(input: &Parser<'_>) -> ByteIndex {
    ByteIndex(input.position().byte_index())
}

/// The target the token `parser` has just read names, resolved against `location`: of a url-token its url, of a `url(` its string,
/// of an `@import` its string or url; nothing for any other token. The parser is set back where it was.
fn target(
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
        found.ok() // ignore: idiomatic.arch/no-ok-on-result - a `url(` whose block holds no string names no target: the parser's refusal is the answer that there is none
    } else if let Token::AtKeyword(name) = token
        && name.eq_ignore_ascii_case(IMPORT)
    {
        let state = parser.state();
        let found = parser
            .expect_url_or_string()
            .ok() // ignore: idiomatic.arch/no-ok-on-result - an `@import` followed by no url or string names no target: the parser's refusal is the answer that there is none
            .map(|url| url.to_string());
        parser.reset(&state);
        found
    } else {
        None
    };
    url.map(|written| languages::resolve_target(location, languages::WrittenTarget(&written)))
}

/// The kind of the token that closes the block `token` opens: its mirror; nothing for a token that opens none.
fn closer(token: Kind) -> Option<Kind> {
    match token {
        Kind::Token(lexeme) => mirror(lexeme).map(Kind::Token),
        Kind::Node(_) | Kind::Extra(_) => None,
    }
}

/// The writer of the table: the library's callbacks, and the tokens between them.
struct Writer<'b, 'w, 't> {
    /// The builder of the file's element table, which every node and token is written through.
    builder: &'b mut languages::TableBuilder<'w>,
    /// The text the parser reads: the file after its byte order mark, up to its first byte that is not UTF-8.
    text: CssText<'t>,
    /// Where the file is, from the root of the run: what its urls are resolved against.
    location: SourcePath<'t>,
    /// Where `text` starts in the file.
    base: ByteIndex,
    /// How far into `text` the tokens are written.
    written: ByteIndex,
    /// Where the last component value the parse met ends: where a node that ends at its content ends.
    content_end: ByteIndex,
    /// The nodes open, the stylesheet first.
    open: Vec<Open>,
    /// The text of a message, kept for the file.
    scratch: Scratch,
}

impl Writer<'_, '_, '_> {
    /// The offset in the file of a place of `text`.
    fn offset(&self, at: ByteIndex) -> Offset {
        Offset(u32::try_from(self.base.0 + at.0).unwrap_or(u32::MAX))
    }

    /// Writes one token of `kind` from `start` to `end` of `text`, with the text the library decoded from it.
    fn put(&mut self, kind: Kind, start: ByteIndex, end: ByteIndex, decoded: Option<NameText<'_>>) {
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
    fn tokens(&mut self, parser: &mut Parser<'_>, shift: ByteIndex, limit: ByteIndex) -> ByteIndex {
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
    fn advance(&mut self, to: ByteIndex) {
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
    fn keyword_start(&self, at: ByteIndex) -> ByteIndex {
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
    fn enter(&mut self, node: Node, at: ByteIndex) {
        self.advance(at);
        languages::enter_node(self.builder, kind_bits(Kind::Node(node)), self.offset(at));
        self.open.push(Open {
            node,
            closing: Closing::AtContent,
        });
    }

    /// Closes the innermost node at `at`, after writing the tokens before it.
    fn leave(&mut self, at: ByteIndex) {
        let at = at.max(self.written);
        self.advance(at);
        languages::leave_node(self.builder, self.offset(at));
        let _: Option<Open> = self.open.pop();
    }

    /// Closes the nodes an item of a list left open, down to `mark`: a rule whose body was opened where the parser is, after its
    /// `}`, and any other node at its content.
    fn close_to(&mut self, mark: Count, at: ByteIndex) {
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
    fn recovered(&mut self, error: &ParseError<()>, slice: Recovered<'_>) {
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
    fn broken(
        &mut self,
        message: languages::ReportText<'static>,
        start: ByteIndex,
        end: ByteIndex,
    ) {
        let (from, to) = (self.offset(start), self.offset(end));
        languages::diagnose(self.builder, message, from, to);
    }

    /// Marks the rule open at the top as one whose body the parser has just entered: it ends after its `}`.
    fn open_body(&mut self) {
        if let Some(rule) = self.open.last_mut() {
            rule.closing = Closing::AtPlace;
        }
    }

    /// Walks the component values `input` gives: it opens the node of each function and simple block, and records what the library
    /// calls a parse error.
    fn components(&mut self, input: &mut Parser<'_>) {
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
    fn rules(&mut self, input: &mut Parser<'_>) {
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
    fn stylesheet(&mut self, input: &mut Parser<'_>) {
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

impl<'i> QualifiedRuleParser<'i> for Writer<'_, '_, '_> {
    type Prelude = ();
    type QualifiedRule = ();
    type Error = ();

    fn parse_prelude(&mut self, input: &mut Parser<'i>) -> Result<(), ParseError<()>> {
        let at = place(input);
        self.enter(Node::QualifiedRule, at);
        self.content_end = at;
        self.components(input);
        Ok(())
    }

    fn parse_block(
        &mut self,
        (): (),
        _start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), ParseError<()>> {
        self.open_body();
        self.rules(input);
        Ok(())
    }
}

impl<'i> AtRuleParser<'i> for Writer<'_, '_, '_> {
    type Prelude = Body;
    type AtRule = ();
    type Error = ();

    fn parse_prelude(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i>,
    ) -> Result<Body, ParseError<()>> {
        // `@charset` is valid only as the stylesheet's first rule, which the library's stylesheet parser reads itself.
        if name.eq_ignore_ascii_case(CHARSET) {
            return Err(ParseError::from_basic_kind(
                BasicParseErrorKind::AtRuleInvalid,
            ));
        }
        let at = place(input);
        let keyword = self.keyword_start(at);
        self.enter(Node::AtRule, keyword);
        self.content_end = at;
        self.components(input);
        Ok(body_of(NameText(&name)))
    }

    fn rule_without_block(&mut self, _body: Body, _start: &ParserState) -> Result<(), ()> {
        Ok(())
    }

    fn parse_block(
        &mut self,
        prelude: Body,
        _start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), ParseError<()>> {
        self.open_body();
        match prelude {
            Body::Rules => self.rules(input),
            Body::Components => self.components(input),
        }
        Ok(())
    }
}

impl<'i> DeclarationParser<'i> for Writer<'_, '_, '_> {
    type Declaration = ();
    type Error = ();

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i>,
        start: &ParserState,
    ) -> Result<(), ParseError<()>> {
        let begin = input.state();
        if !name.starts_with(CUSTOM_PROPERTY) && contents(input) == Contents::HoldsBlock {
            return Err(ParseError::from_basic_kind(
                BasicParseErrorKind::UnexpectedToken,
            ));
        }
        input.reset(&begin);
        self.enter(Node::Declaration, ByteIndex(start.position().byte_index()));
        self.content_end = ByteIndex(begin.position().byte_index());
        self.components(input);
        self.leave(self.content_end);
        Ok(())
    }
}

impl RuleBodyItemParser<'_, (), ()> for Writer<'_, '_, '_> {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        true
    }
}

impl SyntaxPort for SyntaxCssparserAdapter {
    fn read(
        &self,
        text: SourceText<'_>,
        at: SourcePath<'_>,
        builder: &mut languages::TableBuilder<'_>,
    ) {
        // The language reads UTF-8: the library reads the text up to the first byte that is not, and the rest is text it could not read.
        let readable = languages::readable_text(languages::ByteSlice(text.0)).0;
        let (base, sheet) = match readable.strip_prefix(BYTE_ORDER_MARK) {
            Some(rest) => (BYTE_ORDER_MARK.len_utf8(), rest),
            None => (0, readable),
        };
        let length = Offset(u32::try_from(text.0.len()).unwrap_or(u32::MAX));
        languages::enter_node(builder, kind_bits(Kind::Node(Node::Stylesheet)), Offset(0));
        if base > 0 {
            languages::token(
                builder,
                kind_bits(Kind::Extra(Extra::ByteOrderMark)),
                Offset(0),
                Offset(u32::try_from(base).unwrap_or(u32::MAX)),
            );
        }
        let mut writer = Writer {
            builder,
            text: CssText(sheet),
            location: at,
            base: ByteIndex(base),
            written: ByteIndex(0),
            content_end: ByteIndex(0),
            open: vec![Open {
                node: Node::Stylesheet,
                closing: Closing::AtContent,
            }],
            scratch: Scratch::default(),
        };
        let mut parser = Parser::new(sheet);
        writer.stylesheet(&mut parser);
        writer.advance(ByteIndex(sheet.len()));
        let read = writer.offset(ByteIndex(sheet.len()));
        if read < length {
            languages::token(
                writer.builder,
                kind_bits(Kind::Extra(Extra::Error)),
                read,
                length,
            );
            languages::diagnose(
                writer.builder,
                languages::ReportText(NOT_UTF8),
                read,
                length,
            );
        }
        languages::leave_node(writer.builder, length);
    }
}
