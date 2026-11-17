//! What `operator list` promises its caller, one row of `CASES` for each case it has.

use super::entry::list;
use super::errors::ListError;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_core::inbound::use_cases::list_operators::interface::ListOperatorsUseCase as _;

/// How the operators listed compare with the ones the core has: its acceptance holds their count.
#[derive(Debug, PartialEq, Eq)]
enum Listed {
    /// Every operator of the core, each once.
    EveryOperator,
    /// Another number of them.
    Count {
        /// How many the entry listed.
        listed: usize,
        /// How many the core has.
        core: usize,
    },
}

type Input = ();
type Value = Listed;
type Failure = ListError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("lists_every_operator_of_the_core"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Listed::EveryOperator),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let listed = list()?.operators.len();
    let core = super::composition::build()
        .list_operators()
        .map_err(super::schemas::error)?
        .operators
        .len();
    Ok(if listed == core {
        Listed::EveryOperator
    } else {
        Listed::Count { listed, core }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
