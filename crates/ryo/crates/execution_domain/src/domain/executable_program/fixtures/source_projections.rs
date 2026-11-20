//! Source-column views, branch ordering, and the fault controls for their interning.
use super::steps::{compared, per_file, plan, select, text, union_of};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Input as StepInput, LiteralText, StepId, Test,
};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_plans::types::Plan;
use std::sync::Arc;

/// Distinct projections over the same equality scans, with an optional source-column map.
/// The invalid comparison and unread invalid projection must retain their own faults.
pub(super) fn source_views() -> Vec<(Arc<Plan>, Vec<ParameterBinding>)> {
    let make = |view: &[u32], predicate: Test, first: &str, second: &str, columns: &[u32]| {
        let source = StepInput::Relation(Relation::BindingEvent);
        let project = |from, cols: &[u32]| Operation::Project {
            from,
            columns: cols.iter().copied().map(ColumnIndex).collect(),
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Dropped,
        };
        let mut steps = Vec::new();
        let (from, kind, at) = if view.is_empty() {
            (source, 4, 0)
        } else {
            steps.push(per_file(project(source, view)));
            (StepInput::Step(StepId(0)), 2, 1)
        };
        steps.push(per_file(select(
            from,
            vec![compared(predicate, kind, text(first))],
        )));
        steps.push(per_file(select(
            from,
            vec![compared(Test::Equal, kind, text(second))],
        )));
        steps.push(union_of(at, at + 1));
        steps.push(per_file(project(StepInput::Step(StepId(at + 2)), columns)));
        steps.push(per_file(select(
            StepInput::Step(StepId(at + 3)),
            Vec::new(),
        )));
        plan(steps, vec![StepInput::Step(StepId(at + 3))])
    };
    let original = make(&[], Test::Equal, "read", "delete", &[0, 6]);
    vec![
        (Arc::clone(&original), Vec::new()),
        (
            make(&[6, 0, 4], Test::Equal, "read", "delete", &[1, 0]),
            Vec::new(),
        ),
        (
            make(&[6, 0, 4], Test::Equal, "delete", "read", &[1, 0]),
            Vec::new(),
        ),
        (
            make(&[6, 0, 4], Test::Equal, "read", "delete", &[0, 1]),
            Vec::new(),
        ),
        (make(&[], Test::Matches, "[", "delete", &[0, 6]), Vec::new()),
        (
            make(&[6, 0, 4], Test::Matches, "[", "delete", &[1, 0]),
            Vec::new(),
        ),
        (
            original,
            vec![ParameterBinding::Text(LiteralText("elsewhere".into()))],
        ),
        (
            make(&[6, 0, 4, 99], Test::Equal, "read", "delete", &[1, 0]),
            Vec::new(),
        ),
    ]
}
