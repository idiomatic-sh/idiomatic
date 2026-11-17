//! The `scan_bound` item's cases: which lowerings run the kernel.
use super::item::ScanBoundItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::collections::kernels::interface::{KernelsItem as _, Reach};
use crate::domain::physical_plan::types::frontier_place::value::FrontierPlace;
use crate::domain::physical_plan::types::lowering::value::{BoundScan, Lowering};
use crate::domain::physical_plan::types::rule_ordinal::value::RuleOrdinal;
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};
use ryo_values::domain::plan_parts::types::{HostedNumber, InputNumber};

/// The lowering a node was given.
type Input = Lowering;
/// Whether the node runs the kernel.
type Value = Reach;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_of_the_host_s_rows_runs_it"),
        word: Word::Succeeds,
        input: || Lowering::ScanBound {
            scan: BoundScan::Hosted(HostedNumber(0)),
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_filter_of_a_frontier_entry_runs_it"),
        word: Word::Succeeds,
        input: || Lowering::Filter {
            source: RowSource::Frontier(FrontierPlace::of(RuleOrdinal(0), InputNumber(0))),
            select: crate::domain::select::service::lower(&[], &[], &[]),
        },
        expect: Expect::Value(Reach::Runs),
    },
    Case {
        name: CaseName("a_filter_of_a_node_does_not_run_it"),
        word: Word::Succeeds,
        input: || Lowering::Filter {
            source: RowSource::Node(PlanNodeId(0)),
            select: crate::domain::select::service::lower(&[], &[], &[]),
        },
        expect: Expect::Value(Reach::Skips),
    },
];

#[expect(
    clippy::unnecessary_wraps,
    reason = "the unit-test template's subject answers a Result; whether a lowering runs a kernel cannot fail"
)]
fn subject(lowering: Input) -> Result<Value, Failure> {
    Ok(ScanBoundItem.reached_by(&lowering))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
