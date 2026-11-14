//! Preparing two written rules, of which one has a step beyond its operator contract, succeeds, and only that rule's metadata
//! refuses: its emit root and its named-output sources carry the error, while its sibling's roots stay usable. Each rule's
//! plan is the one its author hands over and its parameters are bound from its written spec (RyoQL refuses both authoring
//! errors itself, so a provider is the author that can hand them over; compiling written text is the execution crate's).
use super::super::*;
use super::fixtures::written_rule;
use crate::application::file_dag::program::preparation::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::executable_program::types::InputSource;
use ryo_values::domain::observations::types::{
    ObservationColumn, ObservationColumnName, ObservationName, PlanObservation,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::PlanStep;

/// What preparing a sound rule beside a rule whose last step names a missing column observed.
#[derive(Debug, PartialEq)]
struct Observed {
    /// The sound sibling's emit root error, if it lost its root.
    sibling_emit_root_error: Option<String>,
    /// Whether the errored rule's emit root is an error, though its emit step is sound.
    errored_emit_root_refused: bool,
    /// The errored rule's named outputs, and of those, the ones whose source is that same error.
    named_outputs: usize,
    outputs_with_the_emit_error: usize,
}

/// The column the errored rule's last step projects.
type Input = u32;
type Value = Observed;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("only_the_rule_with_an_errored_step_is_refused_on_its_roots"),
    word: Word::Succeeds,
    input: || 99,
    expect: Expect::Value(Observed {
        sibling_emit_root_error: None,
        errored_emit_root_refused: true,
        // The named output is kept.
        named_outputs: 1,
        outputs_with_the_emit_error: 1,
    }),
}];

/// Prepares the sound rule and the rule whose last step projects the missing column.
fn subject(missing: Input) -> Result<Value, Failure> {
    let sound = plan(ProgramShape::Words);
    let mut errored = plan(ProgramShape::Words);
    // The step after the emit step names a column the select does not keep: the origin is errored, the emit step is sound.
    errored.steps.push(PlanStep {
        operation: Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![ColumnIndex(missing)],
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
    let rules = [
        written_rule("sound", "sound rule"),
        written_rule("errored", "errored rule"),
    ];
    let plans = vec![sound, errored];
    let bindings = plans
        .iter()
        .zip(&rules)
        .map(|(plan, rule)| ryo_values::domain::rule_requests::service::bind(plan, rule))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| format!("{error:?}"))?;
    let compiled = Compiled {
        schedule: RuleSchedule::Independent,
        observations: vec![vec![], vec![]],
        plans: plans.into_iter().map(std::sync::Arc::new).collect(),
        bindings,
    };
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    // An errored rule does not refuse the request: it is refused on its own roots.
    let prepared = prepare(
        compiled,
        &rules,
        &[Digest([0; 16]), Digest([1; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .map_err(|error| format!("{error:?}"))?;
    let sibling = prepared
        .rule_metadata(RuleIndex(0))
        .map_err(|error| format!("{error:?}"))?;
    let refused = prepared
        .rule_metadata(RuleIndex(1))
        .map_err(|error| format!("{error:?}"))?;
    let error = refused.emit_root.clone().err();
    Ok(Observed {
        sibling_emit_root_error: sibling
            .emit_root
            .as_ref()
            .err()
            .map(|error| format!("{error:?}")),
        errored_emit_root_refused: error.is_some(),
        named_outputs: refused.observations.len(),
        outputs_with_the_emit_error: refused
            .observations
            .iter()
            .filter(|output| {
                error
                    .as_ref()
                    .is_some_and(|error| output.source == Err::<InputSource, _>(error.clone()))
            })
            .count(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
