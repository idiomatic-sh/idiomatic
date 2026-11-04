//! Warm selection follows the actual named publication graph, independently of Emit phase.
use super::observations::replay_closure;
use crate::domain::{
    coverage::types::RuleIndex, file_program::types::*, operating::errors::OperatingError,
};
use ryo_values::domain::{
    observations::types::*, plans::types::StepScope, stored_facts::types::Digest,
};

fn binding(producer: usize) -> GraphObservationBinding {
    GraphObservationBinding {
        input: ResolvedObservationBinding {
            input: ObservationBindingId(0),
            schema: BoundObservationSchema {
                name: ObservationBindingName("input".into()),
                columns: Vec::new(),
                scope: StepScope::CrossFile,
            },
            producers: vec![ResolvedObservationProducer::Output {
                rule: RuleIndex(producer),
                output: ObservationIndex(0),
            }],
        },
        union: Err(OperatingError::NotRun), // selection does not execute or inspect an operator
    }
}
fn graph() -> GraphObservations {
    GraphObservations {
        layout: ObservationReadinessLayout(Digest([1; 16])),
        slots: Vec::new(),
        frontiers: Vec::new(),
        publish: (0..4).map(|_| Default::default()).collect(),
        retire: vec![Vec::new(); 4],
        bindings: vec![Vec::new(), vec![binding(0)], vec![binding(1)], Vec::new()],
        outputs: vec![
            vec![GraphObservationOutput {
                root: GraphObservationRoot::CrossFile,
                consumers: vec![1],
            }],
            vec![GraphObservationOutput {
                root: GraphObservationRoot::CrossFile,
                consumers: vec![2],
            }],
            Vec::new(),
            Vec::new(),
        ],
    }
}
#[test]
fn warm_consumer_selects_transitive_cross_producers_and_changed_producer_selects_consumers() {
    let graph = graph();
    let chain = vec![RuleIndex(0), RuleIndex(1), RuleIndex(2)];
    assert_eq!(replay_closure(&graph, &[RuleIndex(2)]).unwrap(), chain);
    assert_eq!(replay_closure(&graph, &[RuleIndex(0)]).unwrap(), chain);
    assert_eq!(
        replay_closure(&graph, &[RuleIndex(3)]).unwrap(),
        vec![RuleIndex(3)]
    );
    assert!(replay_closure(&graph, &[]).unwrap().is_empty());
    assert!(replay_closure(&graph, &[RuleIndex(4)]).is_err());
}
#[test]
fn per_file_producer_is_not_reexecuted_in_the_consumer_cross_phase() {
    let mut graph = graph();
    graph.outputs[0][0].root = GraphObservationRoot::PerFile(NodeId(0));
    assert_eq!(
        replay_closure(&graph, &[RuleIndex(2)]).unwrap(),
        vec![RuleIndex(1), RuleIndex(2)]
    );
}
