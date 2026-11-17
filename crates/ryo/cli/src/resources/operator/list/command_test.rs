//! What `operator list` promises its caller, one row of `CASES` for each case it has.

use super::command::{ListArguments, answer};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::Outcome;
use ryo_core::inbound::use_cases::list_operators::interface::ListOperatorsUseCase as _;

/// How the lines printed compare with the operators the core has: its acceptance holds their count.
#[derive(Debug, PartialEq, Eq)]
enum Printed {
    /// A line for each operator of the core.
    LinePerOperator,
    /// Another number of lines.
    Lines {
        /// How many lines were printed.
        printed: usize,
        /// How many operators the core has.
        core: usize,
    },
}

/// What a row gives the command: the command line, one argument at a time.
type Input = Vec<&'static str>;
/// The lines the command prints when it succeeds, against the operators, and the format its command line chose.
type Value = (Printed, crate::output::Format);
/// How the command ends when it does not succeed.
type Failure = Outcome;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: ListArguments,
    /// The format the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    output: crate::output::FormatSwitch,
}

/// The rows of this command's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("prints_a_line_for_each_operator_of_the_core"),
    word: Word::Succeeds,
    input: || vec![],
    expect: Expect::Value((Printed::LinePerOperator, crate::output::Format::Terminal)),
}];

/// Runs `operator list` on one command line and returns what it printed or the outcome it ended with.
fn subject(line: Input) -> Result<Value, Failure> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("ryo").chain(line))
        .map_err(|_refused| Outcome::Error)?;
    let printed = answer()?.to_string().lines().count();
    let core = super::composition::build()
        .list_operators()
        .map_err(|_failed| Outcome::Error)?
        .operators
        .len();
    let lines = if printed == core {
        Printed::LinePerOperator
    } else {
        Printed::Lines { printed, core }
    };
    Ok((lines, parsed.output.format()))
}

/// The `succeeds` rows: the command line is accepted and the printed answer is the expected one.
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
