//! The batch the cases evaluate, and the oracle of the findings it renders.
use crate::application::cross_file::completion::types::CrossedWithPublications;
use crate::application::cross_file::errors::CrossFileError;
use crate::application::cross_file::fixtures::{World, crossing, kept_facts, service};
use crate::application::cross_file::service::phase_partials;
use crate::application::cross_file::types::ProjectCrossing;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::EmitDemand;
use ryo_execution::inbound::use_cases::execution::interface::FrontierDemand;
use ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles;
use ryo_execution::inbound::use_cases::execution::interface::ObservationDemand;
use ryo_execution::inbound::use_cases::execution::interface::PhaseOccurrence;
use ryo_execution::inbound::use_cases::execution::interface::PhaseOccurrences;
use ryo_execution::inbound::use_cases::execution::interface::PhaseScope;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::PreparedFrontiers;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectInput;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::ProjectTask;
use ryo_execution::inbound::use_cases::execution::interface::PublicationSlot;
use ryo_execution::inbound::use_cases::execution::interface::RuleDemand;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::rule_requests::service::failed;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::stored_facts::types::Bucket;

pub(super) fn completed_batch<T>(
    batch: ProjectBatch<Result<T, PhaseStop<CrossFileError>>, CrossFileError>,
) -> Vec<Result<T, PhaseStop<CrossFileError>>> {
    match batch {
        ProjectBatch::Complete(completed) => completed,
        ProjectBatch::Refused { error, .. } => {
            panic!("the fixture declares no memory limit: {error}")
        }
    }
}

/// The declared occurrences `todos` through the production phase call, over one facts port that keeps each rule's rows in that
/// rule's own first bucket, each outcome completed with its slot.
pub(super) fn evaluate_rules(
    world: &World,
    phase: &impl ProjectRootExecution,
    todos: &[Todo],
    kept: &[(RuleIndex, &[(FileId, Table)])],
    symbols: &mut Symbols,
    guard: impl FnMut(RuleIndex) -> Result<(), CrossFileError>,
) -> ProjectBatch<Result<CrossedWithPublications, PhaseStop<CrossFileError>>, CrossFileError> {
    let crossing = crossing(world);
    let (facts, project) = kept_facts(world, kept);
    let first = (0..world.ids.len())
        .map(|rule| Bucket(u32::try_from(rule).expect("a fixture rule numbers its bucket")))
        .collect::<Vec<_>>();
    let inapplicable = vec![InapplicableFiles::default(); world.ids.len()];
    let occurrences = todos
        .iter()
        .map(|todo| PhaseOccurrence {
            rule: todo.rule,
            bound: &[],
            slot: PublicationSlot::Unused,
            demand: FrontierDemand::Diagnostic,
        })
        .collect::<Vec<_>>();
    let project_symbols = Symbols::default();
    phase.evaluate_occurrences(
        PhaseOccurrences {
            occurrences: &occurrences,
            first: &first,
            corpus: &world.files,
            covered: &world.covered,
            skipped: &world.skipped,
            inapplicable: &inapplicable,
            facts: facts.as_ref(),
            project: project.as_ref(),
            project_symbols: &project_symbols,
            arenas: crossing.arenas,
        },
        symbols,
        |_| Ok::<(), CrossFileError>(()),
        |occurrence| phase_partials(&crossing, &todos[occurrence.0]),
        guard,
        |occurrence, outcome, slot, symbols: &mut Symbols| {
            service().complete_occurrence(&crossing, &todos[occurrence.0], outcome, slot, symbols)
        },
    )
}

/// One occurrence a case prepared over coverage of its own: its one task evaluated through the phase, then completed.
pub(super) fn evaluate_fixture(
    crossing: &ProjectCrossing<'_>,
    todo: &Todo,
    frontier: PreparedFrontiers,
    symbols: &mut Symbols,
    scope: PhaseScope,
    phase: &impl ProjectRootExecution,
) -> Result<crate::domain::answering::types::Crossed, RuleRequestsError> {
    let refused = |error: CrossFileError| match error {
        CrossFileError::Rules(error) => error,
        error => failed(&todo.id, &error),
    };
    let partials = phase_partials(crossing, todo).map_err(refused)?;
    let task = ProjectTask {
        rule: todo.rule,
        demand: RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::Required,
        },
        input: ProjectInput {
            scope: &scope,
            evaluation: &partials.evaluation,
            publication: &partials.publication,
            ordinary: frontier,
            bound: &[],
            arenas: crossing.arenas,
        },
    };
    let ProjectBatch::Complete(mut outcomes) = phase.evaluate(vec![task], symbols, |_| Ok::<(), CrossFileError>(())) else {
        panic!("the fixture declares no memory limit")
    };
    assert_eq!(outcomes.len(), 1, "one authored task has one result");
    let outcome = outcomes
        .pop()
        .expect("the exact result count was checked")
        .map_err(|error| failed(&todo.id, &error))?;
    service()
        .complete_occurrence(crossing, todo, outcome, PublicationSlot::Unused, symbols)
        .map(|result| result.crossed)
        .map_err(refused)
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
