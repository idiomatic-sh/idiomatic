//! Physical ports keep their original IDs and every file stage records its exact source membership.
use super::super::{
    AncestorDistance, EntityAttribute, EntityColumn, EntityExport, EntityExportId, EntityQueryOp,
    EntityStage,
};
use super::value::QueryPlan;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::{RepeatedRows, StepId};
type Input = u32;
type Value = Option<(u32, u8)>;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_held_projection_keeps_the_original_port_and_requires_syntax"),
        word: Word::Succeeds,
        input: || 42,
        expect: Expect::Value(Some((1, 1))),
    },
    Case {
        name: CaseName("a_port_absent_from_the_query_is_not_exposed"),
        word: Word::Succeeds,
        input: || 41,
        expect: Expect::Value(None),
    },
];
fn subject(n: Input) -> Result<Value, Failure> {
    let fields = vec![EntityColumn(EntityAttribute::Id, AncestorDistance(0))];
    let stages = vec![
        EntityStage {
            expr: EntityQueryOp::Syntax,
            fields: fields.clone(),
            operands: vec![],
            physical: None,
        },
        EntityStage {
            expr: EntityQueryOp::Project {
                distinct: RepeatedRows::Dropped,
            },
            fields,
            operands: vec![StepId(0)],
            physical: Some(EntityExportId(42)),
        },
    ];
    let p = QueryPlan::from_stages(stages, vec![EntityExport(EntityExportId(42), StepId(1))]);
    Ok(p.exported(EntityExportId(n))
        .map(|step| (step.0, p.sources[usize::from(step)].0)))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject)
}
