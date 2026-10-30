//! What every `conditions` adapter must pass.

use super::super::interface::ConditionsPort;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::targets::types::Platform;

/// The platform as a manifest writes it.
type Input = &'static str;
/// The platform as Cargo shows it.
type Value = &'static str;
/// Nothing fails: a platform that is not an expression is shown as written.
type Failure = ();

/// The contract's rows. Every implementation runs them.
pub(crate) const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_triple_is_shown_as_it_is"),
        word: Word::Succeeds,
        input: || "x86_64-pc-windows-gnu",
        expect: Expect::Value("x86_64-pc-windows-gnu"),
    },
    Case {
        name: CaseName("a_name_alone_is_shown_without_spaces"),
        word: Word::Succeeds,
        input: || "cfg( unix )",
        expect: Expect::Value("cfg(unix)"),
    },
    Case {
        name: CaseName("a_value_is_shown_with_a_space_each_side_of_the_equals"),
        word: Word::Succeeds,
        input: || "cfg(target_os=\"linux\")",
        expect: Expect::Value("cfg(target_os = \"linux\")"),
    },
    Case {
        name: CaseName("a_group_is_shown_with_a_comma_and_a_space"),
        word: Word::Succeeds,
        input: || "cfg(all(unix,not( target_pointer_width = \"32\" )))",
        expect: Expect::Value("cfg(all(unix, not(target_pointer_width = \"32\")))"),
    },
    Case {
        name: CaseName("an_expression_that_is_not_one_is_shown_as_written"),
        word: Word::Succeeds,
        input: || "cfg(all(unix",
        expect: Expect::Value("cfg(all(unix"),
    },
];

/// Runs the contract's rows against one implementation.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check(adapter: &impl ConditionsPort) {
    run(CASES, Word::Succeeds, |platform| {
        // The answer is leaked: a row compares it with text that lives for the whole run.
        Ok(Box::leak(adapter.show(&Platform(platform.into())).0))
    });
}
