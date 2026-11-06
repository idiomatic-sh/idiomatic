//! The standard's tree of a file, built from `cssparser`'s own reading and apart from the adapter: what the shape rail holds the
//! adapter's tree to. Every token, function and simple block comes from the library's tokenizer and nested blocks, every rule and
//! declaration from its parsers' callbacks; each is nested in the smallest of the others that holds it. The kinds are named here by
//! the standard's names, not through the adapter's schemas.

use cssparser::{AtRuleParser, BasicParseErrorKind, CowRcStr, DeclarationParser, ParseError, Parser, ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, Token};
use ryo::languages::contract::{Leafness, Walked};
use ryo::languages::{NameText, Offset};
use super::super::constants::{BYTE_ORDER_MARK, CHARSET, CUSTOM_PROPERTY, RULE_BODIES};
use super::super::types::{Body, ByteIndex, Contents};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use super::tokens::place;
use super::components::{Depth, Piece, components, content_end, contents, skip_content};

















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
    fn note(&mut self, node: Node, start: ByteIndex, end: ByteIndex) {
        let depth = if node == Node::Declaration {
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
    fn noted(&mut self, node: Node, start: ByteIndex, end: ByteIndex) -> usize {
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
        Ok(self.noted(Node::QualifiedRule, start, end))
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
        self.note(Node::AtRule, place_of(start), end);
        Ok(())
    }

    fn parse_block(
        &mut self,
        (body, _, end): (Body, ByteIndex, ByteIndex),
        start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), ParseError<()>> {
        let rule = self.noted(Node::AtRule, place_of(start), end);
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
        self.note(Node::Declaration, place_of(start), end);
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
        kind: kind_bits(Kind::Node(Node::Stylesheet)),
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
