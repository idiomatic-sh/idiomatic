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
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::rule_requests::errors::RuleRequestsError;
use ryo_values::domain::rule_requests::service::failed;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::stored_facts::types::Bucket;

pub(crate) fn completed_batch<T>(
    batch: ProjectBatch<Result<T, PhaseStop<CrossFileError>>, CrossFileError>,
) -> Vec<Result<T, PhaseStop<CrossFileError>>> {
    match batch {
        ProjectBatch::Complete(completed) => completed,
        ProjectBatch::Refused { error, .. } => vec![Err(PhaseStop::Caller(error))],
    }
}

/// The declared occurrences `todos` through the production phase call, over one facts port that keeps each rule's rows in that
/// rule's own first bucket, each outcome completed with the worker its slot reserved.
pub(crate) fn evaluate_rules(
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
pub(crate) fn evaluate_outcome(
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
pub(crate) fn evaluate_fixture(
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
    match (done.pop(), done.is_empty()) {
        (Some(one), true) => one,
        _ => Err(PhaseStop::Unevaluated),
    }
}

/// The declared occurrences of `rules`, in order, through the production phase call, over one facts port that keeps each rule's
/// `kept` rows in that rule's own first bucket, scoped over `covered`; each outcome handed to `complete`.
pub(crate) fn evaluate_all<T>(
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

/// What a rendered count emit shows, cell by cell: each row's rule, count subject, key and line and column, its text and
/// row ends, and its presentation order as `[rule, line, column, row]`; and the count subjects in the order rendered.
pub(crate) fn count_findings(
    rendered: &crate::domain::render::types::Rendered,
) -> (String, Vec<i64>) {
    use ryo_relations::domain::relations::types::Integer;
    use ryo_values::domain::records::service::lent;
    use ryo_values::domain::row_texts::types::{Cell as RecordCell, CellText, RowsTextView};
    let cells = rendered
        .cells
        .iter()
        .map(
            |cell| match cell.map(|cell| lent(RowsTextView(&rendered.text.0), cell)) {
                Some(RecordCell::Text(CellText(text))) => text.to_owned(),
                Some(RecordCell::Number(Integer(number))) => number.to_string(),
                Some(other) => format!("{other:?}"),
                None => "-".to_owned(),
            },
        )
        .collect::<Vec<_>>();
    let counts = rendered
        .cells
        .chunks_exact(5)
        .filter_map(
            |row| match row[1].map(|cell| lent(RowsTextView(&rendered.text.0), cell)) {
                Some(RecordCell::Number(Integer(count))) => Some(count),
                _ => None,
            },
        )
        .collect::<Vec<_>>();
    let order = rendered
        .order
        .iter()
        .map(|row| {
            format!(
                "{}:{}:{}:{}",
                row.rule.0, row.line.0, row.column.0, row.row.0
            )
        })
        .collect::<Vec<_>>();
    let shown = format!(
        "{} {} {:?} {}",
        cells.join(","),
        rendered.text.0,
        rendered.ends.0,
        order.join(",")
    );
    (shown, counts)
}
