//! The `labels` service's cases: the anchors raw HTML gives.

use super::service::anchors_in;
use super::types::HtmlText;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The raw HTML.
type Input = &'static str;
/// The anchors, joined by a space.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_id_of_a_tag_is_an_anchor"),
        word: Word::Succeeds,
        input: || "<div id=\"top\" class=x>",
        expect: Expect::Value("top"),
    },
    Case {
        name: CaseName("the_name_of_an_a_is_an_anchor_and_the_name_of_another_tag_is_not"),
        word: Word::Succeeds,
        input: || "<a name='here'></a><input name=\"field\">",
        expect: Expect::Value("here"),
    },
    Case {
        name: CaseName("an_unquoted_value_ends_at_a_space"),
        word: Word::Succeeds,
        input: || "<span id=one title=x>",
        expect: Expect::Value("one"),
    },
    Case {
        name: CaseName("a_comment_and_a_closing_tag_give_none"),
        word: Word::Succeeds,
        input: || "<!-- <a id=\"x\"> --></a id=\"y\">",
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("several_anchors_come_in_the_order_they_are_written"),
        word: Word::Succeeds,
        input: || "<a id=\"a\" name=\"b\"></a><p id=\"c\">",
        expect: Expect::Value("a b c"),
    },
];

fn subject(html: Input) -> Value {
    Box::leak(
        anchors_in(HtmlText(html))
            .map(|anchor| anchor.0)
            .collect::<Vec<_>>()
            .join(" ")
            .into_boxed_str(),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
