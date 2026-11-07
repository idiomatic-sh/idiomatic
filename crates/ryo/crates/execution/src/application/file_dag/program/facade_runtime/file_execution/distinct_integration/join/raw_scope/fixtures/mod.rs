//! Inputs are actual completed tree publications; faults are real injected dependencies.
use super::super::capability::fixtures::Witness;
use super::super::{
    cases::test::{tree_rows, wide_tree},
    *,
};
use crate::application::file_dag::derived::owner::fixtures::resources::RecordedDerived;
use ryo_derived::domain::fault::types::DerivedFault;
use ryo_derived::domain::request::types::DerivedRequest;

#[derive(Clone, Copy, Debug)]
pub(super) enum RawCase {
    Complete,
    Streaming,
    SemanticFailure,
    DerivedFailure,
    SinkFailure,
}
pub(super) fn observe(case: RawCase) -> Result<RowCount, OperatingError> {
    let (elements, mut symbols) = wide_tree(1, true);
    let mut request = plan(Scenario::ProjectRetry);
    request.steps = vec![
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                kept: vec![ColumnIndex(0)],
                comparisons: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxChild),
                kept: vec![ColumnIndex(1)],
                comparisons: vec![],
            },
        },
        PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        },
    ];
    request.emit.step = StepId(2);
    let (left, _) = provider(tree_rows(&elements, TreeForm::Elements), 0, &mut vec![]);
    let (right, _) = provider(tree_rows(&elements, TreeForm::Edges), 1, &mut vec![]);
    let derived = Arc::new(if matches!(case, RawCase::DerivedFailure) {
        RecordedDerived::refusing_raw(DerivedFault::DuplicateOrder)
    } else {
        RecordedDerived::default()
    });
    let witness = Arc::new(Witness {
        refuse: matches!(case, RawCase::SemanticFailure),
        ..Witness::default()
    });
    let actual = execute(
        request,
        [left, right],
        &mut symbols,
        b"abcd",
        matches!(case, RawCase::Streaming | RawCase::SinkFailure),
        witness,
        Faults {
            derived,
            optional_limit: Some(0),
            sink_after: matches!(case, RawCase::SinkFailure).then_some(0),
            ..Faults::default()
        },
    )?;
    assert!(
        actual.native_prepared,
        "the real native strategy must be selected"
    );
    assert_eq!(
        actual
            .derived_builds
            .iter()
            .filter(|request| matches!(request, DerivedRequest::RawKeyOrder { stride: ryo_derived::domain::raw_words::types::KeyStride(1) }))
            .count(),
        1,
        "all controls must acquire the actual raw-order kind, even with zero optional retention"
    );
    let rows = actual.result?;
    assert_eq!(rows.cells, [1, 2, 3].map(super::super::cases::test::node));
    Ok(rows.count)
}
