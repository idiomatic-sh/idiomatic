//! Preparing two rules of which one has a step beyond its operator contract succeeds, and only that rule's metadata refuses: its
//! emit root and its named-output sources carry the error, while its sibling's roots stay usable.
use super::super::*;
use crate::application::file_dag::program::preparation::service::prepare;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::InputSource;
use ryo_values::domain::observations::types::{
    ObservationColumn, ObservationColumnName, ObservationName, PlanObservation,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::PlanStep;

#[test]
fn only_the_rule_with_an_errored_step_is_refused_on_its_roots() {
    let sound = plan(Scenario::Words);
    let mut errored = plan(Scenario::Words);
    // The step after the emit step names a column the select does not keep: the origin is errored, the emit step is sound.
    errored.steps.push(PlanStep {
        operation: Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![ColumnIndex(99)],
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        },
        scope: StepScope::PerFile,
    });
    errored.observations = vec![PlanObservation {
        name: ObservationName("kept".into()),
        source: PlanInput::Step(StepId(0)),
        columns: vec![
            ObservationColumn {
                name: ObservationColumnName("node".into()),
                kind: kinds(Relation::SyntaxTree)[0],
            },
            ObservationColumn {
                name: ObservationColumnName("kind".into()),
                kind: kinds(Relation::SyntaxTree)[1],
            },
        ],
        scope: StepScope::PerFile,
    }];
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![], vec![]],
            plans: vec![sound, errored],
            bindings: vec![vec![], vec![]],
        },
        &[rule(), rule()],
        &[Digest([0; 16]), Digest([1; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
        crate::domain::executable_program::types::RoutePolicy::default(),
    )
    .expect("an errored rule does not refuse the request: it is refused on its own roots");
    let sibling = prepared
        .rule_metadata(RuleIndex(0))
        .expect("the sound rule has metadata");
    assert!(
        sibling.emit_root.is_ok(),
        "the sibling rule keeps its emit root"
    );
    let refused = prepared
        .rule_metadata(RuleIndex(1))
        .expect("the errored rule has metadata");
    let error = refused
        .emit_root
        .clone()
        .expect_err("the errored rule's emit root is its error, though its emit step is sound");
    assert_eq!(refused.observations.len(), 1, "the named output is kept");
    for output in refused.observations.iter() {
        assert_eq!(
            output.source,
            Err::<InputSource, _>(error.clone()),
            "the named output's source is the same error"
        );
    }
}
