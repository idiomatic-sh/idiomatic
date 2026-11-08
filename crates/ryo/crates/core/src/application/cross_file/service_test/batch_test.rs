//! The batch the cases evaluate, and the oracle of the findings it renders.
use crate::application::cross_file::completion::types::CrossedWithPublications;
use crate::application::cross_file::errors::CrossFileError;
use crate::application::cross_file::fixtures::service;
use crate::application::cross_file::service::phase_partials;
use crate::application::cross_file::types::ProjectCrossing;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::EmitDemand;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::PreparedFrontiers;
use ryo_execution::inbound::use_cases::execution::interface::ProjectInput;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectTask;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::PhaseScope;
use ryo_execution::inbound::use_cases::execution::interface::PublicationSlot;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::rule_requests::service::failed;
use ryo_values::domain::runs::types::RuleId;

pub(super) fn completed_batch(
    batch: ProjectBatch<Result<CrossedWithPublications, CrossFileError>, CrossFileError>,
) -> Vec<Result<CrossedWithPublications, CrossFileError>> {
    match batch {
        ProjectBatch::Complete(completed) => completed,
        ProjectBatch::Refused { error, .. } => {
            panic!("the fixture declares no memory limit: {error}")
        }
    }
}

/// Occurrences whose scopes and frontiers a case prepared over facts of its own, evaluated as one batch through the phase's
/// `evaluate` and each outcome completed, in declaration order. The phase call prepares from one set of facts; these cases give each
/// rule its own.
pub(super) fn evaluate_prepared(
    crossing: &ProjectCrossing<'_>,
    todos: &[Todo],
    scopes: Vec<PhaseScope>,
    frontiers: Vec<PreparedFrontiers>,
    phase: &impl ProjectRootExecution,
    symbols: &mut Symbols,
    guard: impl FnMut(RuleIndex) -> Result<(), CrossFileError>,
) -> ProjectBatch<Result<CrossedWithPublications, CrossFileError>, CrossFileError> {
    // An occurrence whose partials are refused keeps that refusal and reaches no task.
    let partials = todos
        .iter()
        .map(|todo| phase_partials(crossing, todo))
        .collect::<Vec<_>>();
    let tasks = todos
        .iter()
        .zip(&scopes)
        .zip(frontiers)
        .zip(&partials)
        .filter_map(|(((todo, scope), frontier), partials)| {
            let Ok(partials) = partials else {
                return None;
            };
            Some(ProjectTask {
                rule: todo.rule,
                demand: RuleDemand {
                    emit: EmitDemand::Enabled,
                    observations: ObservationDemand::Required,
                },
                input: ProjectInput {
                    scope,
                    evaluation: &partials.evaluation,
                    publication: &partials.publication,
                    ordinary: frontier,
                    bound: &[],
                    arenas: crossing.arenas,
                },
            })
        })
        .collect();
    let (evaluated, refused) = match phase.evaluate(tasks, symbols, guard) {
        ProjectBatch::Complete(completed) => (completed, None),
        ProjectBatch::Refused { completed, error } => (completed, Some(error)),
    };
    let mut evaluated = evaluated.into_iter();
    let mut completed = Vec::with_capacity(todos.len());
    for (todo, partials) in todos.iter().zip(partials) {
        let result = match partials {
            Err(error) => Err(error),
            Ok(_) => {
                let Some(outcome) = evaluated.next() else {
                    break;
                };
                outcome
                    .map_err(|error| CrossFileError::from(failed(&todo.id, &error)))
                    .and_then(|outcome| service().complete_occurrence(crossing, todo, outcome, PublicationSlot::Unused, symbols))
            }
        };
        completed.push(result);
    }
    match refused {
        Some(error) => ProjectBatch::Refused { completed, error },
        None => ProjectBatch::Complete(completed),
    }
}

/// One occurrence a case prepared, evaluated and completed.
pub(super) fn evaluate_fixture(
    crossing: &ProjectCrossing<'_>,
    todo: &Todo,
    frontier: PreparedFrontiers,
    symbols: &mut Symbols,
    scope: PhaseScope,
    phase: &impl ProjectRootExecution,
) -> Result<crate::domain::answering::types::Crossed, RuleRequestsError> {
    let mut results = completed_batch(evaluate_prepared(
        crossing,
        std::slice::from_ref(todo),
        vec![scope],
        vec![frontier],
        phase,
        symbols,
        |_| Ok(()),
    ));
    assert_eq!(results.len(), 1, "one authored task has one result");
    results
        .pop()
        .expect("the exact result count was checked")
        .map(|result| result.crossed)
        .map_err(|error| match error {
            CrossFileError::Rules(error) => error,
            error => failed(&todo.id, &error),
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
