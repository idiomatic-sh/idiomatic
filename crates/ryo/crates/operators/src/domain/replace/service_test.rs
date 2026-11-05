//! Replacement text is literal, including dollar syntax and unmatched Unicode suffixes.
use super::service::replace_compiled;
use crate::domain::contract::types::PortableOperatorFault;
use ryo_matching::inbound::use_cases::matching::{
    interface::MatchingUseCase, use_case::MatchingUseCaseImpl,
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = (&'static str, &'static str, &'static str);
type Value = std::borrow::Cow<'static, str>;
type Failure = PortableOperatorFault;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("replacement_dollar_is_literal"),
        word: Word::Succeeds,
        input: || ("a", "aaé", "$1"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("$1$1é")),
    },
    Case {
        name: CaseName("zero_width_matches_preserve_characters"),
        word: Word::Succeeds,
        input: || ("^|$", "é", "x"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("xéx")),
    },
    Case {
        name: CaseName("no_matches_preserve_text"),
        word: Word::Succeeds,
        input: || ("x", "é", "y"),
        expect: Expect::Value(std::borrow::Cow::Borrowed("é")),
    },
];
fn subject((pattern, text, replacement): Input) -> Result<Value, Failure> {
    let regex = MatchingUseCaseImpl::new()
        .compile_regex(pattern)
        .expect("valid test pattern");
    let mut output = String::from("stale");
    replace_compiled(&regex, text, replacement, &mut output)?;
    Ok(std::borrow::Cow::Owned(output))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
