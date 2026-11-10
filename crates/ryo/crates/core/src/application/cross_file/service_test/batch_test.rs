//! The batch the cases evaluate, and the oracle of the findings it renders.
use crate::application::cross_file::completion::types::CrossedWithPublications;
use crate::application::cross_file::errors::CrossFileError;
use crate::application::cross_file::fixtures::{World, crossing, kept_facts, service};
use crate::application::cross_file::service::phase_partials;
use crate::domain::answering::types::Todo;
use ryo_execution::inbound::use_cases::execution::interface::InapplicableFiles;
use ryo_execution::inbound::use_cases::execution::interface::OccurrenceNumber;
use ryo_execution::inbound::use_cases::execution::interface::PhasePartials;
use ryo_execution::inbound::use_cases::execution::interface::PhaseRules;
use ryo_execution::inbound::use_cases::execution::interface::PhaseStop;
use ryo_execution::inbound::use_cases::execution::interface::ProjectBatch;
use ryo_execution::inbound::use_cases::execution::interface::ProjectOutputOutcome;
use ryo_execution::inbound::use_cases::execution::interface::ProjectRootExecution;
use ryo_execution::inbound::use_cases::execution::interface::RuleOutcome;
use ryo_execution::inbound::use_cases::metadata::interface::Covered;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{FileId, WorkerNumber};
use ryo_values::domain::stored_facts::types::ArenaBytes;
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
/// rule's own first bucket, each outcome completed with the worker its slot reserved.
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
    let refused = crate::domain::crossing::service::refused_files(&crossing.domain);
    let rules = todos.iter().map(|todo| todo.rule).collect::<Vec<_>>();
    let project_symbols = Symbols::default();
    phase.evaluate_rules(
        PhaseRules {
            rules: &rules,
            first: &first,
            corpus: &world.files,
            covered: &world.covered,
            skipped: &world.skipped,
            inapplicable: &inapplicable,
            refused: &refused,
            facts: facts.as_ref(),
            project: project.as_ref(),
            project_symbols: &project_symbols,
            arenas: crossing.arenas,
        },
        symbols,
        |_| Ok::<(), CrossFileError>(()),
        |occurrence| phase_partials(&crossing.program, &crossing.domain, &todos[occurrence.0]),
        guard,
        |occurrence, outcome, _, worker, symbols: &mut Symbols| {
            service().complete_occurrence(&crossing, &todos[occurrence.0], outcome, worker, symbols)
        },
    )
}

/// The one occurrence of `rule` through the production phase call, scoped over `covered` with `rows` kept in the rule's own first
/// bucket and evaluated with `partials`, its outcome as the engine answered it.
pub(super) fn evaluate_outcome(
    world: &World,
    phase: &impl ProjectRootExecution,
    rule: RuleIndex,
    rows: &[(FileId, Table)],
    covered: &[Covered],
    mut partials: impl FnMut() -> Result<PhasePartials, CrossFileError>,
    symbols: &mut Symbols,
) -> Result<RuleOutcome<ProjectOutputOutcome>, PhaseStop<CrossFileError>> {
    evaluate_one(
        world,
        phase,
        rule,
        rows,
        covered,
        symbols,
        |_| partials(),
        |_, outcome, _, _, _| Ok(outcome),
    )
}

/// One occurrence a case scopes over coverage of its own: evaluated through the production phase call, then completed.
pub(super) fn evaluate_fixture(
    world: &World,
    todo: &Todo,
    rows: &[(FileId, Table)],
    covered: &[Covered],
    symbols: &mut Symbols,
    phase: &impl ProjectRootExecution,
) -> Result<crate::domain::answering::types::Crossed, RuleRequestsError> {
    let crossing = crossing(world);
    let refused = |error: CrossFileError| match error {
        CrossFileError::Rules(error) => error,
        error @ (CrossFileError::Budget(_)
        | CrossFileError::Memory(_)
        | CrossFileError::Operating(_)
        | CrossFileError::Facts(_)
        | CrossFileError::Scheduler(_)) => failed(&todo.id, &error),
    };
    let done = evaluate_one(
        world,
        phase,
        todo.rule,
        rows,
        covered,
        symbols,
        |_| phase_partials(&crossing.program, &crossing.domain, todo),
        |_, outcome, _, worker, symbols: &mut Symbols| {
            service().complete_occurrence(&crossing, todo, outcome, worker, symbols)
        },
    );
    done.map(|result| result.crossed)
        .map_err(|stop| refused(crate::application::cross_file::service::stopped(todo, stop)))
}

/// The one occurrence of `rule` through the production phase call, its outcome handed to `complete`.
fn evaluate_one<T>(
    world: &World,
    phase: &impl ProjectRootExecution,
    rule: RuleIndex,
    rows: &[(FileId, Table)],
    covered: &[Covered],
    symbols: &mut Symbols,
    partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, CrossFileError>,
    complete: impl FnMut(
        OccurrenceNumber,
        RuleOutcome<ProjectOutputOutcome>,
        &[ArenaBytes],
        Option<WorkerNumber>,
        &mut Symbols,
    ) -> Result<T, CrossFileError>,
) -> Result<T, PhaseStop<CrossFileError>> {
    let mut done = evaluate_all(
        world,
        phase,
        &[rule],
        &[(rule, rows)],
        covered,
        symbols,
        partials,
        complete,
    );
    assert_eq!(done.len(), 1, "one declared occurrence has one result");
    done.pop().expect("the exact result count was checked")
}

/// The declared occurrences of `rules`, in order, through the production phase call, over one facts port that keeps each rule's
/// `kept` rows in that rule's own first bucket, scoped over `covered`; each outcome handed to `complete`.
pub(super) fn evaluate_all<T>(
    world: &World,
    phase: &impl ProjectRootExecution,
    rules: &[RuleIndex],
    kept: &[(RuleIndex, &[(FileId, Table)])],
    covered: &[Covered],
    symbols: &mut Symbols,
    partials: impl FnMut(OccurrenceNumber) -> Result<PhasePartials, CrossFileError>,
    complete: impl FnMut(
        OccurrenceNumber,
        RuleOutcome<ProjectOutputOutcome>,
        &[ArenaBytes],
        Option<WorkerNumber>,
        &mut Symbols,
    ) -> Result<T, CrossFileError>,
) -> Vec<Result<T, PhaseStop<CrossFileError>>> {
    let (facts, project) = kept_facts(world, kept);
    let first = (0..world.ids.len())
        .map(|each| Bucket(u32::try_from(each).expect("a fixture rule numbers its bucket")))
        .collect::<Vec<_>>();
    let inapplicable = vec![InapplicableFiles::default(); world.ids.len()];
    let refused_files = vec![
            ryo_execution::inbound::use_cases::execution::interface::RefusedFiles::default();
            world.ids.len()
        ];
    let project_symbols = Symbols::default();
    let batch = phase.evaluate_rules(
        PhaseRules {
            rules,
            first: &first,
            corpus: &world.files,
            covered,
            skipped: &world.skipped,
            inapplicable: &inapplicable,
            refused: &refused_files,
            facts: facts.as_ref(),
            project: project.as_ref(),
            project_symbols: &project_symbols,
            arenas: &world.arenas,
        },
        symbols,
        |_| Ok::<(), CrossFileError>(()),
        partials,
        |_| Ok(()),
        complete,
    );
    completed_batch(batch)
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
