//! The library's own reading of a stylesheet, compared with the rows: `cssparser`'s rule and declaration parsers, given parsers of
//! their own that only note what they are called for, say which rules and declarations the stylesheet has, where each starts and in
//! which order, and what each `@import` imports (`Parser::expect_url_or_string`, compared by the file it names). What the oracle
//! rail of the conformance holds `syntax_tree` and `static_import` to.

use cssparser::{
    AtRuleParser, BasicParseErrorKind, CowRcStr, DeclarationParser, ParseError, Parser,
    ParserState, QualifiedRuleParser, RuleBodyItemParser, RuleBodyParser, StyleSheetParser, Token,
};
use ryo::languages::Relation;
use ryo::languages::contract::{Cell, OracleRows, Point, point_of};

use super::super::constants::{BYTE_ORDER_MARK, CHARSET, CUSTOM_PROPERTY, IMPORT};
use super::super::types::Body;
use super::walk::body_of;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_name;
use crate::domain::syntax::types::Kind;

/// What the library's parsers were called for, in the order of the file.
struct Noted {
    /// Each rule and declaration: its kind and where it starts in the text read.
    items: Vec<(Node, usize)>,
    /// What each `@import` imports.
    imports: Vec<String>,
    /// How many rules and declarations the library recovered from.
    errors: usize,
}

/// Skips what `input` holds.
fn skip(input: &mut Parser<'_>) {
    while input.next_including_whitespace_and_comments().is_ok() {}
}

impl Noted {
    /// Reads the rules and declarations of a block.
    fn body(&mut self, input: &mut Parser<'_>) {
        let results: Vec<bool> = RuleBodyParser::<_, (), ()>::new(input, self)
            .map(|each| each.is_ok())
            .collect();
        self.errors += results.iter().filter(|each| !**each).count();
    }
}

impl<'i> QualifiedRuleParser<'i> for Noted {
    type Prelude = ();
    type QualifiedRule = ();
    type Error = ();

    fn parse_prelude(&mut self, input: &mut Parser<'i>) -> Result<(), ParseError<()>> {
        skip(input);
        Ok(())
    }

    fn parse_block(
        &mut self,
        (): (),
        start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), ParseError<()>> {
        self.items
            .push((Node::QualifiedRule, start.position().byte_index()));
        self.body(input);
        Ok(())
    }
}

impl<'i> AtRuleParser<'i> for Noted {
    type Prelude = (Body, Option<String>);
    type AtRule = ();
    type Error = ();

    fn parse_prelude(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i>,
    ) -> Result<(Body, Option<String>), ParseError<()>> {
        if name.eq_ignore_ascii_case(CHARSET) {
            return Err(ParseError::from_basic_kind(
                BasicParseErrorKind::AtRuleInvalid,
            ));
        }
        let import = if name.eq_ignore_ascii_case(IMPORT) {
            input
                .expect_url_or_string()
                .ok()
                .map(|target| target.to_string())
        } else {
            None
        };
        skip(input);
        Ok((body_of(ryo::languages::NameText(&name)), import))
    }

    fn rule_without_block(
        &mut self,
        (_, import): (Body, Option<String>),
        start: &ParserState,
    ) -> Result<(), ()> {
        self.items
            .push((Node::AtRule, start.position().byte_index()));
        self.imports.extend(import);
        Ok(())
    }

    fn parse_block(
        &mut self,
        (body, import): (Body, Option<String>),
        start: &ParserState,
        input: &mut Parser<'i>,
    ) -> Result<(), ParseError<()>> {
        self.items
            .push((Node::AtRule, start.position().byte_index()));
        self.imports.extend(import);
        match body {
            Body::Rules => self.body(input),
            Body::Components => skip(input),
        }
        Ok(())
    }
}

impl<'i> DeclarationParser<'i> for Noted {
    type Declaration = ();
    type Error = ();

    fn parse_value(
        &mut self,
        name: CowRcStr<'i>,
        input: &mut Parser<'i>,
        start: &ParserState,
    ) -> Result<(), ParseError<()>> {
        let mut tokens = Vec::new();
        while let Ok(token) = input.next() {
            tokens.push(matches!(token, Token::CurlyBracketBlock));
        }
        if !name.starts_with(CUSTOM_PROPERTY) && tokens.len() > 1 && tokens.contains(&true) {
            return Err(ParseError::from_basic_kind(
                BasicParseErrorKind::UnexpectedToken,
            ));
        }
        self.items
            .push((Node::Declaration, start.position().byte_index()));
        Ok(())
    }
}

impl RuleBodyItemParser<'_, (), ()> for Noted {
    fn parse_declarations(&self) -> bool {
        true
    }

    fn parse_qualified(&self) -> bool {
        true
    }
}

/// The text of a name cell; nothing for any other.
fn name(cell: Option<&Cell>) -> Option<&str> {
    match cell {
        Some(Cell::Name(text)) => Some(text),
        _ => None,
    }
}

/// Where a span cell starts.
fn starts(cell: Option<&Cell>) -> Option<Point> {
    match cell {
        Some(Cell::Span(from, _)) => Some(*from),
        _ => None,
    }
}

/// Compares the rules, declarations and imports the rows of `text` say with what the library reads: what differs, one sentence each;
/// nothing when they agree. The rules and declarations are compared only in a stylesheet the library recovers nothing from, where
/// each is the library's whole.
pub(crate) fn oracle(text: &str, rows: &OracleRows) -> Vec<String> {
    let (base, read) = match text.strip_prefix(BYTE_ORDER_MARK) {
        Some(rest) => (BYTE_ORDER_MARK.len_utf8(), rest),
        None => (0, text),
    };
    let mut noted = Noted {
        items: Vec::new(),
        imports: Vec::new(),
        errors: 0,
    };
    let mut input = Parser::new(read);
    let results: Vec<bool> = StyleSheetParser::new(&mut input, &mut noted)
        .map(|each| each.is_ok())
        .collect();
    noted.errors += results.iter().filter(|each| !**each).count();
    let mut differences = Vec::new();
    let structural = [Node::QualifiedRule, Node::AtRule, Node::Declaration]
        .map(|kind| kind_name(Kind::Node(kind)).0);
    if noted.errors == 0 {
        let said: Vec<(String, Point)> = rows
            .of(Relation::SyntaxTree)
            .iter()
            .filter_map(|row| {
                let kind = name(row.get(1))?;
                if structural.contains(&kind) {
                    Some((kind.to_owned(), starts(row.get(4))?))
                } else {
                    None
                }
            })
            .collect();
        let library: Vec<(String, Point)> = noted
            .items
            .iter()
            .map(|(kind, start)| {
                let at = u32::try_from(base + start).unwrap_or(u32::MAX);
                (
                    kind_name(Kind::Node(*kind)).0.to_owned(),
                    point_of(text.as_bytes(), at),
                )
            })
            .collect();
        if said != library {
            differences.push(format!(
                "the rows say {} rules and declarations, the library reads {}: the first that differs is {:?} against {:?}",
                said.len(),
                library.len(),
                said.iter().zip(&library).find(|(ours, theirs)| ours != theirs).map(|(ours, _)| ours),
                said.iter().zip(&library).find(|(ours, theirs)| ours != theirs).map(|(_, theirs)| theirs),
            ));
        }
    }
    let imported: Vec<String> = rows
        .of(Relation::StaticImport)
        .iter()
        .filter_map(|row| name(row.get(3)).map(str::to_owned))
        .collect();
    // The rows say the path of the run an import names; the library, the url as written. Each says the same file by its last part,
    // and a url with a scheme or a host stays as it is written.
    let last = |path: &str| path.rsplit('/').next().unwrap_or(path).to_owned();
    if imported.iter().map(|each| last(each)).collect::<Vec<_>>()
        != noted
            .imports
            .iter()
            .map(|each| last(each))
            .collect::<Vec<_>>()
    {
        differences.push(format!(
            "the rows import {imported:?}, the library reads {:?}",
            noted.imports
        ));
    }
    differences
}
