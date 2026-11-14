//! What every `matching` adapter must pass.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::matching::interface::MatchingPort;
use crate::selects;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::interface::MatchingUseCase as _;
use ryo_matching::inbound::use_cases::matching::types::{GroupName, RegexSource, SearchText};

/// What a row of the contract is given: the pattern, the text, and the group asked for, when one is.
type Input = (&'static str, &'static str, Option<&'static str>);
/// The occurrences a row finds: each as a start and an end, with the group's start and end beside it when one was asked for.
type Value = std::borrow::Cow<'static, [(usize, usize, Option<(usize, usize)>)]>;
/// What a row of the contract expects when it is refused.
type Failure = MatchingError;

/// The contract's rows: a name, one case word, an input and the expected value or error variant. Every implementation runs them.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("occurrences_are_visited_in_source_order"),
        word: Word::Succeeds,
        input: || ("a", "banana", None),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            (1, 2, None),
            (3, 4, None),
            (5, 6, None),
        ])),
    },
    Case {
        name: CaseName("a_text_without_the_pattern_has_no_occurrences"),
        word: Word::Succeeds,
        input: || ("z", "banana", None),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("a_named_group_is_visited_with_its_occurrence"),
        word: Word::Succeeds,
        input: || ("b(?<rest>an)", "banana", Some("rest")),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[(0, 3, Some((1, 3)))])),
    },
    Case {
        name: CaseName("a_group_the_pattern_lacks_is_refused"),
        word: Word::Refuses,
        input: || ("a", "banana", Some("rest")),
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, MatchingError::InvalidRegex { .. }))
        }),
    },
];

/// Runs one input of the contract through one implementation.
///
/// # Errors
///
/// `InvalidRegex` when the pattern has no group of the name asked for.
pub fn answer(
    adapter: &impl MatchingPort,
    (pattern, text, group): Input,
) -> Result<Value, Failure> {
    let regex = adapter.semantics().compile_regex(RegexSource(pattern))?;
    let mut found = Vec::new();
    match group {
        None => {
            let _flow = adapter.visit_regex_matches(&regex, SearchText(text), &mut |span| {
                found.push((span.0.start, span.0.end, None));
                std::ops::ControlFlow::Continue(())
            });
        }
        Some(name) => {
            let _flow = adapter.visit_regex_captures(
                &regex,
                SearchText(text),
                GroupName(name),
                &mut |span, group| {
                    found.push((span.0.start, span.0.end, Some((group.0.start, group.0.end))));
                    std::ops::ControlFlow::Continue(())
                },
            )?;
        }
    }
    Ok(std::borrow::Cow::Owned(found))
}

/// Runs the contract's rows against one implementation: the fake, or an adapter of another crate.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl MatchingPort) {
    run(CASES, Word::Succeeds, |input: Input| answer(adapter, input));
    run(CASES, Word::Refuses, |input: Input| answer(adapter, input));
}
