//! The run budget, through the plan and run use cases with counting off: a making that writes far more rows than it was fed
//! stops the run naming its node and its rows; one past the floor but within the rows per row fed is kept.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::inbound::use_cases::run::fixtures::budget::one_name_product;
use std::borrow::Cow;

/// How many Defines rows of one name the file holds: the self join on the name writes the square of it.
type Input = u32;
/// The rows the join wrote, or the fault, and the run's worst making.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_making_past_the_budget_fails_naming_its_rows_and_the_worst_making_its_node"),
        word: Word::Succeeds,
        input: || 600,
        expect: Expect::Value(Cow::Borrowed(
            "Err(OverBudget { fed: RowCount(1200), wrote: RowCount(360000) }) worst Some(WorstMaking { node: PlanNodeId(1), fed: RowCount(1200), wrote: RowCount(360000) })",
        )),
    },
    Case {
        name: CaseName("a_making_past_the_floor_within_the_rows_per_row_fed_is_kept"),
        word: Word::Succeeds,
        input: || 300,
        expect: Expect::Value(Cow::Borrowed(
            "Ok([RowCount(90000)]) worst Some(WorstMaking { node: PlanNodeId(1), fed: RowCount(600), wrote: RowCount(90000) })",
        )),
    },
];

/// Every Defines row of one name joined with every other on the name, in one file, counting off.
fn subject(rows: Input) -> Result<Value, Failure> {
    let mut buffers = FileBuffers::default();
    let answered = one_name_product(rows, &mut buffers);
    Ok(Cow::Owned(format!(
        "{:?} worst {:?}",
        answered.map(|tables| tables.iter().map(|table| table.count).collect::<Vec<_>>()),
        buffers.worst_making()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
