//! The subprocess adapter of the tools port: the contract, the environment a tool sees, a tool killed at its timeout,
//! and an empty command.

use super::adapter::ToolsSubprocessAdapter;
use super::connection::ToolsConnection;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::manifest::types::{Argument, Argv, ProductRoot, Seconds};
use crate::outbound::tools::fixtures::contract::check;
use crate::outbound::tools::interface::ToolsPort as _;
use crate::outbound::tools::types::{Ending, Invocation};

/// Which question is asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// The contract.
    Contract,
    /// What a tool sees of the environment.
    Environment,
    /// A tool that runs past its timeout.
    Timeout,
    /// An empty command.
    Empty,
    /// A tool whose group holds a second process, stopped through its connection while it runs.
    Stopped,
}

type Input = Question;
/// Whether the answer is the expected one.
type Value = bool;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_adapter_keeps_the_contract"),
        word: Word::Succeeds,
        input: || Question::Contract,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_tool_runs_in_the_c_locale_without_the_callers_rustflags"),
        word: Word::Succeeds,
        input: || Question::Environment,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_tool_past_its_timeout_is_killed"),
        word: Word::Succeeds,
        input: || Question::Timeout,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("a_running_tool_is_stopped_with_its_group_through_its_connection"),
        word: Word::Succeeds,
        input: || Question::Stopped,
        expect: Expect::Value(true),
    },
    Case {
        name: CaseName("an_empty_command_could_not_start"),
        word: Word::Succeeds,
        input: || Question::Empty,
        expect: Expect::Value(true),
    },
];

/// The adapter over a connection of its own.
fn adapter() -> ToolsSubprocessAdapter {
    ToolsSubprocessAdapter {
        connection: ToolsConnection::new(),
    }
}

/// Whether a tool whose group holds a second process ends, signalled, when its connection is stopped.
fn stopped() -> bool {
    let connection = ToolsConnection::new();
    let running = ToolsSubprocessAdapter {
        connection: connection.clone(),
    };
    let started = std::time::Instant::now();
    let ran = std::thread::spawn(move || running.run(&shell("sleep 60 & sleep 60; wait", 120)));
    while connection.first().is_none() && started.elapsed().as_secs() < 10 {
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    connection.stop();
    let ended = ran.join().expect("the run ends");
    ended.ending == Ending::Signalled && started.elapsed().as_secs() < 30
}

fn here() -> ProductRoot {
    ProductRoot(env!("CARGO_MANIFEST_DIR").into())
}

fn shell(script: &str, timeout: u64) -> Invocation {
    Invocation {
        argv: Argv(
            ["sh", "-c", script]
                .into_iter()
                .map(|argument| Argument(argument.into()))
                .collect(),
        ),
        directory: here(),
        timeout: Seconds(timeout),
    }
}

fn subject(question: Input) -> Value {
    match question {
        Question::Contract => {
            check(&adapter(), &here());
            true
        }
        Question::Environment => {
            let said = adapter().run(&shell(
                "printf '%s %s' \"$LC_ALL\" \"${RUSTFLAGS-unset}\"",
                30,
            ));
            &*said.stdout.0 == "C unset"
        }
        Question::Timeout => adapter().run(&shell("sleep 30", 0)).ending == Ending::TimedOut,
        Question::Stopped => stopped(),
        Question::Empty => matches!(
            adapter()
                .run(&Invocation {
                    argv: Argv(Vec::new()),
                    directory: here(),
                    timeout: Seconds(1),
                })
                .ending,
            Ending::Unstartable(_)
        ),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
