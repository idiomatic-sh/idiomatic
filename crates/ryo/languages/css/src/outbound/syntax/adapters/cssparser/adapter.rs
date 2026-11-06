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

use super::constants::{BYTE_ORDER_MARK, CHARSET, CUSTOM_PROPERTY, NOT_UTF8};
use super::types::{Body, ByteIndex, Closing, Contents, CssText, Open, Scratch};
use super::writer::service::{Writer, body_of, contents, place};
use crate::domain::extras::types::Extra;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::service::kind_bits;
use crate::domain::syntax::types::Kind;
use crate::outbound::syntax::interface::SyntaxPort;
use crate::outbound::syntax::types::SourceText;
use cssparser::{
    AtRuleParser, BasicParseErrorKind, CowRcStr, DeclarationParser, ParseError, Parser,
    ParserState, QualifiedRuleParser, RuleBodyItemParser,
};
use ryo::languages::SourcePath;
use ryo::languages::{self, NameText, Offset};

/// The `cssparser` syntax adapter.
#[derive(Debug)]
pub(crate) struct SyntaxCssparserAdapter;

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
