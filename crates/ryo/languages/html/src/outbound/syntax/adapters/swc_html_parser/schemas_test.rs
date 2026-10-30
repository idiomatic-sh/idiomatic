//! The native tokenizer census is either mapped or explicitly absent at EOF.
use super::token_kind;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::{service::kind_name, types::Kind};
use swc_common::{BytePos, input::StringInput};
use swc_html_ast::Token;
use swc_html_parser::lexer::Lexer;
#[derive(Clone, Copy, Debug)]
enum Given {
    Vocabulary,
    Every,
    End,
}
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    Names(&'static str),
    Accounted(bool),
    Dropped(Option<&'static str>),
}
type Input = Given;
type Value = Seen;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_native_token_category_has_its_own_kind"),
        word: Word::Succeeds,
        input: || Given::Vocabulary,
        expect: Expect::Value(Seen::Names(
            "doctype-token start-tag characters end-tag comment-token",
        )),
    },
    Case {
        name: CaseName("the_complete_native_token_census_is_accounted_for"),
        word: Word::Succeeds,
        input: || Given::Every,
        expect: Expect::Value(Seen::Accounted(true)),
    },
    Case {
        name: CaseName("end_of_input_has_no_source_token"),
        word: Word::Succeeds,
        input: || Given::End,
        expect: Expect::Value(Seen::Dropped(Some("EOF has no source bytes"))),
    },
];
fn library_kinds() -> Vec<Token> {
    let source = "<!doctype html><p>x</p><!--c-->";
    Lexer::new(StringInput::new(
        source,
        BytePos(1),
        BytePos(u32::try_from(source.len()).unwrap_or(u32::MAX) + 1),
    ))
    .map(|token| token.token)
    .chain(std::iter::once(Token::Eof))
    .collect()
}
fn dropped(kind: &Token) -> Option<&'static str> {
    match kind {
        Token::Eof => Some("EOF has no source bytes"),
        Token::Doctype { .. }
        | Token::StartTag { .. }
        | Token::EndTag { .. }
        | Token::Comment { .. }
        | Token::Character { .. } => None,
    }
}
fn accounted(kind: &Token) -> bool {
    token_kind(kind).is_some() || dropped(kind).is_some()
}
fn subject(input: Input) -> Value {
    match input {
        Given::Vocabulary => Seen::Names(Box::leak(
            library_kinds()
                .iter()
                .filter_map(token_kind)
                .map(|kind| kind_name(Kind::Token(kind)).0)
                .collect::<Vec<_>>()
                .join(" ")
                .into_boxed_str(),
        )),
        Given::Every => Seen::Accounted(library_kinds().iter().all(accounted)),
        Given::End => Seen::Dropped(dropped(&Token::Eof)),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
