//! The `existence_probe` item's cases, and the collection's shared cases when it has any: The `existence_probe` rewrite stops a semi or anti probe at the first right row of a key, and leaves an inner probe alone.
use super::item::ExistenceProbeItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::equality_join::types::{Existence, ProbeMode};
use crate::domain::physical_plan::collections::rewrites::interface::RewritesItem as _;
use crate::domain::physical_plan::types::{
    Lowering, NodeReaders, PhysicalNode, PlanNodeId, Retention, RowSource,
};
use crate::domain::set_rows::types::{SetKind, SetLowering};

/// Which set meaning's probe is rewritten.
type Input = SetKind;
/// How the probe decides that a key has a right row after the rewrite.
type Value = Existence;
type Failure = ();

/// The rows of this part's cases: a name, one case word, an input from the fixtures or a constructor, and the expected value or error variant.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_antijoin_stops_at_the_first_match"),
        word: Word::Succeeds,
        input: || SetKind::Antijoin,
        expect: Expect::Value(Existence::First),
    },
    Case {
        name: CaseName("an_intersection_stops_at_the_first_match"),
        word: Word::Succeeds,
        input: || SetKind::Intersection,
        expect: Expect::Value(Existence::First),
    },
];

/// Calls the part on one input; the author states it.
fn subject(kind: Input) -> Result<Value, Failure> {
    let SetLowering::Probe(probe) = crate::domain::set_rows::service::lower(kind, &[], &[]) else {
        return Err(());
    };
    let mut inner = probe.clone();
    inner.mode = ProbeMode::Inner;
    let mut nodes: Vec<PhysicalNode> = [probe, inner]
        .into_iter()
        .map(|probe| PhysicalNode {
            lowering: Lowering::Probe {
                left: RowSource::Node(PlanNodeId(0)),
                right: RowSource::Node(PlanNodeId(1)),
                probe,
            },
            readers: NodeReaders(0),
            retained: Retention::Retained,
        })
        .collect();
    ExistenceProbeItem.rewrite(&mut nodes, &mut Vec::new());
    let existence = |node: &PhysicalNode| match &node.lowering {
        Lowering::Probe { probe, .. } => Some(probe.existence),
        _ => None,
    };
    // An inner probe reads every right row of its key: the rewrite leaves it.
    match (
        nodes.first().and_then(existence),
        nodes.get(1).and_then(existence),
    ) {
        (Some(rewritten), Some(Existence::Every)) => Ok(rewritten),
        _ => Err(()),
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
