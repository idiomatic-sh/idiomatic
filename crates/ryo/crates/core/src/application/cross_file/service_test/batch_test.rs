//! The batch the cases evaluate, and the oracle of the findings it renders.
use crate::application::cross_file::completion::types::CrossedWithPublications;
use crate::application::cross_file::errors::CrossFileError;
use crate::application::cross_file::fixtures::service;
use crate::application::cross_file::preparation::service::SealedBatch;
use crate::application::cross_file::types::ProjectCrossing;
use crate::application::cross_file::types::ProjectOutcome;
use crate::domain::answering::types::Todo;
use ryo_execution::application::file_dag::frontiers::types::PreparedFrontiers;
use ryo_execution::application::file_dag::program::rule_outcome::types::ProjectBatch;
use ryo_execution::application::file_dag::program::project_phase::types::ProjectScope;
use ryo_execution::application::file_dag::frontiers::types::PublicationSlot;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::runs::types::RuleId;

pub(super) fn completed_batch(
    batch: ProjectBatch<ProjectOutcome, CrossFileError>,
) -> Vec<Result<CrossedWithPublications, CrossFileError>> {
    match batch {
        ProjectBatch::Complete(completed) => completed
            .into_iter()
            .map(|outcome| outcome.result)
            .collect(),
        ProjectBatch::Refused { error, .. } => {
            panic!("the fixture declares no memory limit: {error}")
        }
    }
}

/// Test resources enter through the actual immutable declaration owner.
pub(super) fn sealed_fixture_batch(
    todo: &[Todo],
    scopes: Vec<ProjectScope>,
    frontiers: Vec<PreparedFrontiers>,
) -> SealedBatch {
    use crate::application::cross_file::preparation::{
        service::Preparation, types::PreparedProject,
    };
    let mut resources = scopes.into_iter().zip(frontiers);
    let prepared = Preparation::ordinary(todo).prepare_all(|_, _| {
        let (scope, frontier) = resources
            .next()
            .expect("fixture prepares every declared occurrence");
        Ok(PreparedProject {
            scope,
            frontier,
            bound: Vec::new(),
            slot: PublicationSlot::Unused,
        })
    });
    assert!(
        resources.next().is_none(),
        "fixture has no undeclared resources"
    );
    prepared.seal().unwrap()
}

/// Fixture assembly only: the sole application batch operation owns admission/evaluation.
pub(super) fn evaluate_fixture(
    crossing: &ProjectCrossing<'_>,
    todo: &Todo,
    frontier: PreparedFrontiers,
    symbols: &mut Symbols,
    scope: ProjectScope,
    phase: &impl ryo_execution::application::file_dag::program::project_phase::interface::ProjectRootExecution,
) -> Result<crate::domain::answering::types::Crossed, RuleRequestsError> {
    let batch = sealed_fixture_batch(std::slice::from_ref(todo), vec![scope], vec![frontier]);
    let ProjectBatch::Complete(mut results) =
        service().evaluate_batch(crossing, batch, phase, symbols, |_| Ok(()))
    else {
        panic!("fixture host declares no memory limit")
    };
    assert_eq!(results.len(), 1, "one authored task has one result");
    results
        .pop()
        .expect("the exact result count was checked")
        .result
        .map(|result| result.crossed)
        .map_err(|error| match error {
            CrossFileError::Rules(error) => error,
            error => ryo_values::domain::rule_requests::service::failed(&todo.id, &error),
        })
}

/// Independent Findings-layout oracle for the authored count emit (no keys or headings).
/// The grouping input arrives as count 2 then count 1; the rendered presentation
/// orders subjects numerically, retaining each rule's own header and row identity.
pub(super) fn assert_count_findings(
    rendered: &crate::domain::render::types::Rendered,
    rule: RuleIndex,
    id: &RuleId,
    counts: &[i64],
    presentation: &[u32],
) -> Vec<i64> {
    use ryo_relations::domain::relations::types::Integer;
    use ryo_values::domain::records::service::lent;
    use ryo_values::domain::row_texts::types::{Cell as RecordCell, CellText, RowsTextView};
    let observed = rendered
        .cells
        .iter()
        .map(|cell| cell.map(|cell| lent(RowsTextView(&rendered.text.0), cell)))
        .collect::<Vec<_>>();
    let expected = counts
        .iter()
        .flat_map(|&count| {
            [
                Some(RecordCell::Text(CellText(&id.0))),
                Some(RecordCell::Number(Integer(count))),
                Some(RecordCell::Number(Integer(0))),
                None,
                None,
            ]
        })
        .collect::<Vec<_>>();
    assert_eq!(
        observed, expected,
        "rule, count subject, empty key, and unplaced line/column"
    );
    assert_eq!(rendered.text.0, id.0.repeat(counts.len()));
    assert_eq!(
        rendered.ends.0,
        (1..=counts.len())
            .map(|row| u32::try_from(row * 5).unwrap())
            .collect::<Vec<_>>()
    );
    assert_eq!(
        rendered
            .order
            .iter()
            .map(|row| [row.rule.0, row.line.0, row.column.0, row.row.0])
            .collect::<Vec<_>>(),
        presentation
            .iter()
            .map(|&row| [u32::try_from(rule.0).unwrap(), 0, 0, row])
            .collect::<Vec<_>>()
    );
    observed
        .chunks_exact(5)
        .map(|row| match row[1] {
            Some(RecordCell::Number(Integer(count))) => count,
            _ => panic!("count subject must remain a typed number"),
        })
        .collect()
}
