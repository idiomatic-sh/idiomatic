//! Unicode, acronym, digit and punctuation policies remain explicit.
use super::service::split;
use crate::domain::words::types::WordScratch;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::row_texts::types::CellText;
type Input = &'static str;
type Value = std::borrow::Cow<'static, str>;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("case_digit_and_acronym_boundaries"),
        word: Word::Succeeds,
        input: || "HTTPServer2Go",
        expect: Expect::Value(std::borrow::Cow::Borrowed("httpserver2|go")),
    },
    Case {
        name: CaseName("punctuation_and_unicode_lowercase"),
        word: Word::Succeeds,
        input: || "ÜberName_one-two 42Cats",
        expect: Expect::Value(std::borrow::Cow::Borrowed("über|name|one|two|42|cats")),
    },
    Case {
        name: CaseName("separators_never_emit_empty_words"),
        word: Word::Succeeds,
        input: || "_- .",
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
];
fn subject(text: Input) -> Result<Value, Failure> {
    let mut word = String::with_capacity(32);
    let mut output = Vec::new();
    split(CellText(text), WordScratch(&mut word), &mut |word: &str| {
        output.push(word.to_owned())
    });
    assert!(word.is_empty());
    Ok(std::borrow::Cow::Owned(output.join("|")))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
