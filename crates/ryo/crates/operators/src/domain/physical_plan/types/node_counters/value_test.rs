//! Counts add up node by node across workers; a making that wrote no row takes the time of its open span and of the stage
//! spans before it as empty time, and a making that wrote rows takes none; a span's time leaves out the spans nested in it.
use super::value::{MadeIn, NodeCounters};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::{NodeFed, PlanNodeId};
use ryo_values::domain::rows::types::RowCount;
use std::borrow::Cow;

/// What a case does to the counters.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// Node 0 made in an open span with rows, node 1 in an open span with none, node 1 nested in node 0's span.
    Nested,
    /// Node 2 a chain stage timed before it is made with no row, then node 2 made with rows after another timed push.
    Stages,
    /// Two workers' counts of node 0, merged.
    Merged,
}
type Input = Given;
/// One line per node counted: its makings, its empty makings, its rows read and written, and how much of its time is empty.
type Value = Cow<'static, str>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_making_with_no_row_takes_its_span_as_empty_time"),
        word: Word::Succeeds,
        input: || Given::Nested,
        expect: Expect::Value(Cow::Borrowed(
            "0: runs 1 empty 0 in 4 out 2 empty time none\n1: runs 1 empty 1 in 4 out 0 empty time all",
        )),
    },
    Case {
        name: CaseName("a_chain_stage_takes_the_time_before_its_making"),
        word: Word::Succeeds,
        input: || Given::Stages,
        expect: Expect::Value(Cow::Borrowed(
            "2: runs 2 empty 1 in 8 out 5 empty time part",
        )),
    },
    Case {
        name: CaseName("merged_counts_add_up"),
        word: Word::Succeeds,
        input: || Given::Merged,
        expect: Expect::Value(Cow::Borrowed(
            "0: runs 2 empty 1 in 8 out 1 empty time none\n1: runs 1 empty 0 in 4 out 4 empty time none",
        )),
    },
];

/// What node 0 was fed in each case: three rows on its first input, one on its second.
const FED: NodeFed = NodeFed([RowCount(3), RowCount(1), RowCount(0)]);

/// Keeps a span open long enough that its time is not zero.
fn busy() {
    let started = std::time::Instant::now();
    while started.elapsed().is_zero() {
        std::hint::spin_loop();
    }
}

fn subject(given: Input) -> Result<Value, Failure> {
    let mut counters = NodeCounters::default();
    match given {
        Given::Nested => {
            let outer = counters.opened();
            busy();
            let inner = counters.opened();
            busy();
            counters.made(PlanNodeId(1), MadeIn::OpenSpan, FED, RowCount(0));
            counters.closed(inner, PlanNodeId(1));
            counters.made(PlanNodeId(0), MadeIn::OpenSpan, FED, RowCount(2));
            counters.closed(outer, PlanNodeId(0));
        }
        Given::Stages => {
            let span = counters.opened();
            busy();
            counters.closed(span, PlanNodeId(2));
            counters.made(PlanNodeId(2), MadeIn::Chain, FED, RowCount(0));
            let span = counters.opened();
            busy();
            counters.closed(span, PlanNodeId(2));
            counters.made(PlanNodeId(2), MadeIn::Chain, FED, RowCount(5));
        }
        Given::Merged => {
            let mut other = NodeCounters::default();
            counters.made(PlanNodeId(0), MadeIn::Chain, FED, RowCount(1));
            other.made(PlanNodeId(0), MadeIn::Chain, FED, RowCount(0));
            other.made(PlanNodeId(1), MadeIn::Chain, FED, RowCount(4));
            counters.merge(other);
        }
    }
    Ok(Cow::Owned(
        counters
            .counts()
            .map(|counted| {
                let work = counted.work;
                let empty = if work.empty_time.is_zero() {
                    "none"
                } else if work.empty_time == work.time {
                    "all"
                } else {
                    "part"
                };
                format!(
                    "{}: runs {} empty {} in {} out {} empty time {empty}",
                    counted.node.0, work.runs.0, work.empty_runs.0, work.rows_in.0, work.rows_out.0
                )
            })
            .collect::<Vec<_>>()
            .join("\n"),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
