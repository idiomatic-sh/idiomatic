//! A node reads its sources in the order its kernels read them.
use super::value::Lowering;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::physical_plan::types::{PlanNodeId, RowSource};

/// The lowering of a node.
type Input = Lowering;
/// The rows it reads, in order.
type Value = std::borrow::Cow<'static, [RowSource]>;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_union_reads_its_left_then_its_right"),
        word: Word::Succeeds,
        input: || Lowering::Distinct {
            sources: [
                RowSource::Node(PlanNodeId(0)),
                RowSource::Node(PlanNodeId(1)),
            ],
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[
            RowSource::Node(PlanNodeId(0)),
            RowSource::Node(PlanNodeId(1)),
        ])),
    },
    Case {
        name: CaseName("an_unlowered_node_reads_nothing"),
        word: Word::Succeeds,
        input: || Lowering::Unlowered,
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[])),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |lowering: Input| {
        Ok(lowering.sources().collect())
    });
}
