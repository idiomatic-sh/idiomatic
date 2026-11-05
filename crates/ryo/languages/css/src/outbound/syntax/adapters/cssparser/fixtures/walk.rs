// the walk is the library's rule parsers' receiver and the standard's names for its tokens, written apart from the adapter on purpose (library-walk-is-its-own): the second implementation is the proof, and a split would be a file no adapter template places
//! The standard's tree of a file, built from `cssparser`'s own reading and apart from the adapter: what the shape rail holds the
//! adapter's tree to. Every token, function and simple block comes from the library's tokenizer and nested blocks, every rule and
//! declaration from its parsers' callbacks; each is nested in the smallest of the others that holds it. The kinds are named here by
//! the standard's names, not through the adapter's schemas.

use cssparser::{
    AtRuleParser, BasicParseErrorKind, CowRcStr, DeclarationParser, ParseError, Parser,
    ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, Token,
};
use ryo::languages::contract::{Leafness, Walked};
use ryo::languages::{NameText, Offset};

use super::super::constants::{BYTE_ORDER_MARK, CHARSET, CUSTOM_PROPERTY, RULE_BODIES};
use super::super::types::{Body, ByteIndex, Contents};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::CssNode;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::service::mirror;
use crate::domain::tokens::types::Token as Lexeme;

/// Where the parser is, in bytes from the start of its text.
fn place(input: &Parser<'_>) -> ByteIndex {
    ByteIndex(input.position().byte_index())
}

/// The standard's token of the library's, or the extra a comment is.
fn token_kind(token: &Token<'_>) -> Kind {
    Kind::Token(match token {
        Token::Ident(_) => Lexeme::Ident,
        Token::AtKeyword(_) => Lexeme::AtKeyword,
        Token::Hash(_) | Token::IDHash(_) => Lexeme::Hash,
        Token::QuotedString(_) => Lexeme::String,
        Token::UnquotedUrl(_) => Lexeme::Url,
        Token::Delim(_)
        | Token::IncludeMatch
        | Token::DashMatch
        | Token::PrefixMatch
        | Token::SuffixMatch
        | Token::SubstringMatch => Lexeme::Delim,
        Token::Number { .. } => Lexeme::Number,
        Token::Percentage { .. } => Lexeme::Percentage,
        Token::Dimension { .. } => Lexeme::Dimension,
        Token::WhiteSpace(_) => Lexeme::Whitespace,
        Token::Comment(_) => return Kind::Extra(Extra::Comment),
        Token::Colon => Lexeme::Colon,
        Token::Semicolon => Lexeme::Semicolon,
        Token::Comma => Lexeme::Comma,
        Token::CDO => Lexeme::Cdo,
        Token::CDC => Lexeme::Cdc,
        Token::Function(_) => Lexeme::Function,
        Token::ParenthesisBlock => Lexeme::LeftParenthesis,
        Token::SquareBracketBlock => Lexeme::LeftSquare,
        Token::CurlyBracketBlock => Lexeme::LeftCurly,
        Token::BadUrl(_) => Lexeme::BadUrl,
        Token::BadString(_) => Lexeme::BadString,
        Token::CloseParenthesis => Lexeme::RightParenthesis,
        Token::CloseSquareBracket => Lexeme::RightSquare,
        Token::CloseCurlyBracket => Lexeme::RightCurly,
    })
}

/// Whether the token is white space or a comment.
const fn is_trivia(token: &Token<'_>) -> bool {
    matches!(token, Token::WhiteSpace(_) | Token::Comment(_))
}

/// The node a token opens: a function or a simple block.
const fn opens(token: &Token<'_>) -> Option<CssNode> {
    match token {
        Token::Function(_) => Some(CssNode::Function),
        Token::ParenthesisBlock | Token::SquareBracketBlock | Token::CurlyBracketBlock => {
            Some(CssNode::SimpleBlock)
        }
        Token::Ident(_)
        | Token::AtKeyword(_)
        | Token::Hash(_)
        | Token::IDHash(_)
        | Token::QuotedString(_)
        | Token::UnquotedUrl(_)
        | Token::Delim(_)
        | Token::Number { .. }
        | Token::Percentage { .. }
        | Token::Dimension { .. }
        | Token::WhiteSpace(_)
        | Token::Comment(_)
        | Token::Colon
        | Token::Semicolon
        | Token::Comma
        | Token::IncludeMatch
        | Token::DashMatch
        | Token::PrefixMatch
        | Token::SuffixMatch
        | Token::SubstringMatch
        | Token::CDO
        | Token::CDC
        | Token::BadUrl(_)
        | Token::BadString(_)
        | Token::CloseParenthesis
        | Token::CloseSquareBracket
        | Token::CloseCurlyBracket => None,
    }
}

/// Whether the library reads the token as one where the standard reads two delim-tokens: an attribute matcher.
const fn is_matcher(token: &Token<'_>) -> bool {
    matches!(
        token,
        Token::IncludeMatch
            | Token::DashMatch
            | Token::PrefixMatch
            | Token::SuffixMatch
            | Token::SubstringMatch
    )
}

/// What the block of the at-rule `name` holds: rules for an at-rule CSS defines so, with or without a vendor prefix.
pub(super) fn body_of(name: NameText<'_>) -> Body {
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
fn contents(input: &mut Parser<'_>) -> Contents {
    let (mut block, mut content) = (false, 0_usize);
    while let Ok(token) = input.next() {
        block |= matches!(token, Token::CurlyBracketBlock);
        content += 1;
    }
    if block && content > 1 {
        Contents::HoldsBlock
    } else {
        Contents::Plain
    }
}

/// How deep a piece nests among pieces that start at the same place.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Depth {
    /// A rule.
    Rule,
    /// A declaration.
    Declaration,
    /// A function or a simple block.
    Component,
    /// A token.
    Token,
}

/// A piece of the tree before it is nested: its kind, where it lies, and how it nests.
#[derive(Clone, Copy, Debug)]
struct Piece {
    /// What it is: a node or a token of the stylesheet.
    kind: Kind,
    /// Where it starts in the text read.
    start: ByteIndex,
    /// Where it ends in the text read.
    end: ByteIndex,
    /// How deep it nests, which orders two pieces that start together.
    depth: Depth,
}

/// Every token, function and simple block of `parser`'s text, from the library's tokenizer and its nested blocks.
fn components(parser: &mut Parser<'_>, pieces_out: &mut Vec<Piece>) {
    loop {
        let start = place(parser);
        let Ok(token) = parser.next_including_whitespace_and_comments().cloned() else {
            return;
        };
        let end = place(parser);
        let kind = token_kind(&token);
        let opened = opens(&token);
        let node = pieces_out.len();
        if let Some(opened) = opened {
            pieces_out.push(Piece {
                kind: Kind::Node(opened),
                start,
                end,
                depth: Depth::Component,
            });
        }
        if is_matcher(&token) {
            let middle = ByteIndex(start.0 + 1);
            pieces_out.push(token_piece(kind, start, middle));
            pieces_out.push(token_piece(kind, middle, end));
        } else {
            pieces_out.push(token_piece(kind, start, end));
        }
        if opened.is_some() {
            let before = pieces_out.len();
            let nested: Result<(), ParseError<()>> = parser.parse_nested_block(|inner| {
                components(inner, pieces_out);
                Ok(())
            });
            let after = place(parser);
            let inside = pieces_out
                .get(before..)
                .unwrap_or_default()
                .iter()
                .map(|each| each.end)
                .max()
                .unwrap_or(end);
            match (nested, kind) {
                (Ok(()), Kind::Token(lexeme)) if after > inside => {
                    if let Some(closing) = mirror(lexeme) {
                        pieces_out.push(token_piece(Kind::Token(closing), inside, after));
                    }
                }
                (Err(_), _) if after > end => {
                    pieces_out.push(token_piece(Kind::Extra(Extra::Error), end, after));
                }
                _ => {}
            }
            if let Some(piece) = pieces_out.get_mut(node) {
                piece.end = after;
            }
        }
    }
}

/// A token piece.
const fn token_piece(kind: Kind, start: ByteIndex, end: ByteIndex) -> Piece {
    Piece {
        kind,
        start,
        end,
        depth: Depth::Token,
    }
}

/// Where a run of component values ends: after its last one that is not white space or a comment; `from` for none.
fn content_end(input: &mut Parser<'_>, from: ByteIndex) -> ByteIndex {
    let mut end = from;
    loop {
        let Ok(token) = input.next_including_whitespace_and_comments() else {
            return end;
        };
        if is_trivia(token) {
            continue;
        }
        if opens(token).is_some() {
            // The block is read only to pass over it: whether its contents parse does not move where the run ends.
            drop(input.parse_nested_block(|inner| {
                skip_content(inner);
                Ok::<(), ParseError<()>>(())
            }));
        }
        end = place(input);
    }
}

/// Passes over the rest of a run of component values, nested blocks included.
fn skip_content(input: &mut Parser<'_>) {
    while input.next_including_whitespace_and_comments().is_ok() {}
}

/// The rules and declarations of a stylesheet, as the library's parsers are called for them.
struct Rules {
    /// The rules and declarations noted so far.
    pieces: Vec<Piece>,
    /// The rules whose body the item of the list being read entered: each ends where the parser is after the item.
    bodied: Vec<usize>,
    /// Where each body a rule entered starts, at its `{`: a body is the rule's, no simple block.
    bodies: Vec<ByteIndex>,
    /// The runs of component values the callbacks are handed: a function or a simple block is a node only in one.
    runs: Vec<(ByteIndex, ByteIndex)>,
}

impl Rules {
    /// Notes a rule or a declaration.
    fn note(&mut self, node: CssNode, start: ByteIndex, end: ByteIndex) {
        let depth = if node == CssNode::Declaration {
            Depth::Declaration
        } else {
            Depth::Rule
        };
        self.pieces.push(Piece {
            kind: Kind::Node(node),
            start,
            end,
            depth,
        });
    }

    /// Notes a rule whose end is moved later, past its body: the place of its piece, by which it is moved.
    fn noted(&mut self, node: CssNode, start: ByteIndex, end: ByteIndex) -> usize {
        self.note(node, start, end);
        self.pieces.len() - 1
    }

    /// Reads the items of a body.
    fn body(&mut self, input: &mut Parser<'_>) {
        self.bodies
            .push(ByteIndex(place(input).0.saturating_sub(1)));
        let mut parser = RuleBodyParser::<_, (), ()>::new(input, self);
        loop {
            let mark = parser.parser.bodied.len();
            let Some(_) = parser.next() else {
                return;
            };
            let at = place(parser.input);
            for each in parser.parser.bodied.split_off(mark) {
                if let Some(piece) = parser.parser.pieces.get_mut(each) {
                    piece.end = at;
                }
            }
        }
    }
}

impl<'i> QualifiedRuleParser<'i> for Rules {
    type Prelude = usize;
    type QualifiedRule = ();
    type Error = ();

    fn parse_prelude(&mut self, input: &mut Parser<'i>) -> Result<usize, ParseError<()>> {
        let start = place(input);
        let end = content_end(input, start);
        self.runs.push((start, place(input)));
        Ok(self.noted(CssNode::QualifiedRule, start, end))
    }

    fn parse_block(
        &mut self,
        prelude: usize,
        _start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), ParseError<()>> {
        self.bodied.push(prelude);
        self.body(input);
        Ok(())
    }
}

impl<'i> AtRuleParser<'i> for Rules {
    type Prelude = (Body, ByteIndex, ByteIndex);
    type AtRule = ();
    type Error = ();

    fn parse_prelude(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i>,
    ) -> Result<(Body, ByteIndex, ByteIndex), ParseError<()>> {
        if name.eq_ignore_ascii_case(CHARSET) {
            return Err(ParseError::from_basic_kind(
                BasicParseErrorKind::AtRuleInvalid,
            ));
        }
        let from = place(input);
        let end = content_end(input, from);
        self.runs.push((from, place(input)));
        Ok((body_of(NameText(&name)), from, end))
    }

    fn rule_without_block(
        &mut self,
        (_, _, end): (Body, ByteIndex, ByteIndex),
        start: &ParserState,
    ) -> Result<(), ()> {
        self.note(CssNode::AtRule, place_of(start), end);
        Ok(())
    }

    fn parse_block(
        &mut self,
        (body, _, end): (Body, ByteIndex, ByteIndex),
        start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), ParseError<()>> {
        let rule = self.noted(CssNode::AtRule, place_of(start), end);
        self.bodied.push(rule);
        match body {
            Body::Rules => self.body(input),
            Body::Components => {
                self.bodies
                    .push(ByteIndex(place(input).0.saturating_sub(1)));
                let from = place(input);
                skip_content(input);
                self.runs.push((from, place(input)));
            }
        }
        Ok(())
    }
}

impl<'i> DeclarationParser<'i> for Rules {
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
        let end = content_end(input, place_of(&begin));
        self.runs.push((place_of(&begin), place(input)));
        self.note(CssNode::Declaration, place_of(start), end);
        Ok(())
    }
}

impl RuleBodyItemParser<'_, (), ()> for Rules {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        true
    }
}

/// Where a state of the parser is.
fn place_of(state: &ParserState) -> ByteIndex {
    ByteIndex(state.position().byte_index())
}

/// An offset of the file, `base` bytes before the text read.
fn offset(base: ByteIndex, at: ByteIndex) -> Offset {
    Offset(u32::try_from(base.0 + at.0).unwrap_or(u32::MAX))
}

/// Closes the innermost open piece into the one that holds it, or into the root when none does.
fn close(open: &mut Vec<(Walked, ByteIndex)>, root: &mut Walked) {
    if let Some((done, _)) = open.pop() {
        match open.last_mut() {
            Some((parent, _)) => parent.children.push(done),
            None => root.children.push(done),
        }
    }
}

/// The standard's tree of `text`: the stylesheet, its rules and declarations, and their component values and tokens, each nested
/// in the smallest of the others that holds it.
#[must_use]
pub(crate) fn walked(text: &str) -> Walked {
    let (base, read) = match text.strip_prefix(BYTE_ORDER_MARK) {
        Some(rest) => (ByteIndex(BYTE_ORDER_MARK.len_utf8()), rest),
        None => (ByteIndex(0), text),
    };
    let mut found = Vec::new();
    components(&mut Parser::new(read), &mut found);
    let mut rules = Rules {
        pieces: Vec::new(),
        bodied: Vec::new(),
        bodies: Vec::new(),
        runs: Vec::new(),
    };
    let mut input = Parser::new(read);
    let mut sheet = StyleSheetParser::new(&mut input, &mut rules);
    loop {
        let mark = sheet.parser.bodied.len();
        let Some(_) = sheet.next() else {
            break;
        };
        let at = place(sheet.input);
        for each in sheet.parser.bodied.split_off(mark) {
            if let Some(piece) = sheet.parser.pieces.get_mut(each) {
                piece.end = at;
            }
        }
    }
    found.retain(|each| {
        each.depth != Depth::Component
            || (!rules.bodies.contains(&each.start)
                && rules
                    .runs
                    .iter()
                    .any(|(from, to)| *from <= each.start && each.start < *to))
    });
    found.extend(rules.pieces);
    found.sort_by_key(|each| (each.start, std::cmp::Reverse(each.end), each.depth));
    let mut root = Walked {
        kind: kind_bits(Kind::Node(CssNode::Stylesheet)),
        leaf: Leafness::Node,
        start: Offset(0),
        end: offset(base, ByteIndex(read.len())),
        children: Vec::new(),
    };
    if base.0 > 0 {
        root.children.push(Walked {
            kind: kind_bits(Kind::Extra(Extra::ByteOrderMark)),
            leaf: Leafness::Token,
            start: Offset(0),
            end: offset(ByteIndex(0), base),
            children: Vec::new(),
        });
    }
    let mut open: Vec<(Walked, ByteIndex)> = Vec::new();
    for piece in found {
        while open.last().is_some_and(|(_, end)| *end <= piece.start) {
            close(&mut open, &mut root);
        }
        let leaf = if piece.depth == Depth::Token {
            Leafness::Token
        } else {
            Leafness::Node
        };
        let here = Walked {
            kind: kind_bits(piece.kind),
            leaf,
            start: offset(base, piece.start),
            end: offset(base, piece.end),
            children: Vec::new(),
        };
        match (leaf, open.last_mut()) {
            (Leafness::Node, _) => open.push((here, piece.end)),
            (Leafness::Token, Some((parent, _))) => parent.children.push(here),
            (Leafness::Token, None) => root.children.push(here),
        }
    }
    while !open.is_empty() {
        close(&mut open, &mut root);
    }
    root
}
