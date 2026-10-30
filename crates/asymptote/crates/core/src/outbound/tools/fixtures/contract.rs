//! What every `tools` adapter must pass: a run's exit code and output come back, and a program that is not there is
//! one that could not start.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::{Argument, Argv, ProductRoot, Seconds};
use crate::outbound::tools::interface::ToolsPort;
use crate::outbound::tools::types::{Ending, ExitCode, Invocation};

/// The command line.
type Input = Vec<&'static str>;
/// How it ended, and its stdout.
type Value = std::borrow::Cow<'static, str>;
/// None: a run always completes, its failures told in how it ended.
type Failure = ();

/// The contract's rows: every implementation runs them, given the commands `echo` and `false` and no `missing`.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_run_that_exits_answers_its_code_and_output"),
        word: Word::Succeeds,
        input: || vec!["echo", "out"],
        expect: Expect::Value(std::borrow::Cow::Borrowed("0 out")),
    },
    Case {
        name: CaseName("a_run_that_fails_answers_its_code"),
        word: Word::Succeeds,
        input: || vec!["false"],
        expect: Expect::Value(std::borrow::Cow::Borrowed("1 ")),
    },
    Case {
        name: CaseName("a_program_that_is_not_there_could_not_start"),
        word: Word::Succeeds,
        input: || vec!["/nonexistent/missing-tool"],
        expect: Expect::Value(std::borrow::Cow::Borrowed("unstartable")),
    },
];

/// Runs the contract's rows against one implementation, in `directory`.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub fn check(adapter: &impl ToolsPort, directory: &ProductRoot) {
    run(CASES, Word::Succeeds, |argv| -> Result<Value, Failure> {
        let completion = adapter.run(&Invocation {
            argv: Argv(
                argv.into_iter()
                    .map(|argument| Argument(argument.into()))
                    .collect(),
            ),
            directory: directory.clone(),
            timeout: Seconds(30),
        });
        Ok(std::borrow::Cow::Owned(match completion.ending {
            Ending::Exited(ExitCode(code)) => format!("{code} {}", completion.stdout.0.trim()),
            Ending::Unstartable(_) => "unstartable".to_owned(),
            Ending::Signalled | Ending::TimedOut => "ended".to_owned(),
        }))
    });
}
