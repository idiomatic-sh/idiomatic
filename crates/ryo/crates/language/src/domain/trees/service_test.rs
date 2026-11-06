//! The `trees` service's cases, on values alone: a tree written from events and gaps spells the whole file, and a library reads the
//! text up to its first byte that is not UTF-8.

use super::service::{
    fill_to, lossless, lossless_decoded_token, lossless_enter, lossless_leave, lossless_token,
    readable_text,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::elements::service::{end_at, kind_at, positions, start_at};
use crate::domain::builders::service::{enter_node, leave_node, table_builder, token};
use crate::domain::decoded::service::decoded_at;
use crate::domain::elements::types::{Elements, TableBuilder};
use crate::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;

/// The kind the cases' events write.
const EVENT: KindBits = KindBits(1);
/// The kind the cases' gap tokenizer writes.
const GAP: KindBits = KindBits(2);
/// The kind of the root the cases open.
const ROOT: KindBits = KindBits(3);
/// The kind of a node an event opens.
const NODE: KindBits = KindBits(4);

/// A scenario: what a row asks of the service, shown as text.
type Input = fn() -> String;
/// What it shows: every element below the root, `kind:start-end`, and `=text` for a decoded token; or the text a library reads.
type Value = &'static str;
type Failure = ();

/// The cases' gap tokenizer: one token of the gap kind over the whole stretch.
fn gap(builder: &mut TableBuilder<'_>, from: Offset, to: Offset) {
    token(builder, GAP, from, to);
}

/// Writes `events` over `text` below a root and shows every element below it.
fn written(text: &[u8], events: fn(&mut TableBuilder<'_>)) -> String {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(text));
    enter_node(&mut builder, ROOT, Offset(0));
    events(&mut builder);
    leave_node(
        &mut builder,
        Offset(u32::try_from(text.len()).unwrap_or(u32::MAX)),
    );
    positions(&elements)
        .skip(1)
        .map(|at| {
            let decoded =
                decoded_at(&elements, at).map_or(String::new(), |value| format!("={}", value.0));
            format!(
                "{}:{}-{}{decoded}",
                kind_at(&elements, at).0,
                start_at(&elements, at).0,
                end_at(&elements, at).0
            )
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn two_tokens_and_the_space_between() -> String {
    written(b"a  b", |builder| {
        let mut tree = lossless(builder, Offset(0));
        lossless_token(&mut tree, EVENT, Offset(0), Offset(1), &mut gap);
        lossless_token(&mut tree, EVENT, Offset(3), Offset(4), &mut gap);
        fill_to(&mut tree, Offset(4), &mut gap);
    })
}

fn a_token_that_starts_before_the_last_ends() -> String {
    written(b"abc", |builder| {
        let mut tree = lossless(builder, Offset(0));
        lossless_token(&mut tree, EVENT, Offset(0), Offset(2), &mut gap);
        lossless_token(&mut tree, EVENT, Offset(1), Offset(3), &mut gap);
    })
}

fn a_node_between_gaps() -> String {
    written(b" [x] ", |builder| {
        let mut tree = lossless(builder, Offset(0));
        lossless_enter(&mut tree, NODE, Offset(1), &mut gap);
        lossless_token(&mut tree, EVENT, Offset(2), Offset(3), &mut gap);
        lossless_leave(&mut tree, Offset(4), &mut gap);
        fill_to(&mut tree, Offset(5), &mut gap);
    })
}

fn a_decoded_token_and_the_text_after_it() -> String {
    written(b"ab", |builder| {
        let mut tree = lossless(builder, Offset(0));
        lossless_decoded_token(
            &mut tree,
            EVENT,
            Offset(0),
            Offset(1),
            NameText("A"),
            &mut gap,
        );
        fill_to(&mut tree, Offset(2), &mut gap);
    })
}

fn a_tree_that_starts_past_a_mark() -> String {
    written(b"#ab", |builder| {
        let mut tree = lossless(builder, Offset(1));
        lossless_token(&mut tree, EVENT, Offset(1), Offset(3), &mut gap);
    })
}

fn text_that_is_utf8() -> String {
    readable_text(ByteSlice("a\u{e9}b".as_bytes())).0.to_owned()
}

fn text_with_a_byte_that_is_not_utf8() -> String {
    readable_text(ByteSlice(b"ab\xffcd")).0.to_owned()
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_space_between_two_tokens_is_a_gap_token"),
        word: Word::Succeeds,
        input: || two_tokens_and_the_space_between,
        expect: Expect::Value("1:0-1 2:1-3 1:3-4"),
    },
    Case {
        name: CaseName("a_token_that_starts_before_the_last_ends_starts_where_it_ends"),
        word: Word::Succeeds,
        input: || a_token_that_starts_before_the_last_ends,
        expect: Expect::Value("1:0-2 1:2-3"),
    },
    Case {
        name: CaseName("a_node_opens_after_the_gap_before_it_and_holds_the_gap_before_its_end"),
        word: Word::Succeeds,
        input: || a_node_between_gaps,
        expect: Expect::Value("2:0-1 4:1-4 2:1-2 1:2-3 2:3-4 2:4-5"),
    },
    Case {
        name: CaseName("a_decoded_token_keeps_its_text_and_the_text_after_the_last_event_is_a_gap"),
        word: Word::Succeeds,
        input: || a_decoded_token_and_the_text_after_it,
        expect: Expect::Value("1:0-1=A 2:1-2"),
    },
    Case {
        name: CaseName("a_tree_whose_tokens_start_past_a_mark_has_no_gap_before_them"),
        word: Word::Succeeds,
        input: || a_tree_that_starts_past_a_mark,
        expect: Expect::Value("1:1-3"),
    },
    Case {
        name: CaseName("text_that_is_utf8_is_read_whole"),
        word: Word::Succeeds,
        input: || text_that_is_utf8,
        expect: Expect::Value("a\u{e9}b"),
    },
    Case {
        name: CaseName("text_is_read_up_to_its_first_byte_that_is_not_utf8"),
        word: Word::Succeeds,
        input: || text_with_a_byte_that_is_not_utf8,
        expect: Expect::Value("ab"),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(Box::leak(scenario().into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
