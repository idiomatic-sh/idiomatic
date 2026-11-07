//! The lifecycle cases of a lint: unchanged, edited, deleted, failed and many runs over one store, and where a finding is placed.
use super::super::interface::LintUseCase as _;
type Failure = LintError;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::row_sink::fixtures::fake::Ends;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::row_texts::types::Cell as Typed;
use ryo_values::domain::stored_findings::types::FindingMade;
use ryo_values::domain::stores::types::Stored;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use super::given::Given;
use super::given::SOURCES;
use super::given::World;
use super::given::request;
use super::given::violations;
use super::super::errors::LintError;
use super::worlds::again;
use super::worlds::lint;
use super::worlds::read_and_unread;
use super::worlds::world;

/// Lints three files with a rule that counts the files each function name is in, edits `a.rs` to `edit`, and lints again over the
/// same store: answers how many files the second lint read, and 1 when the cross-file findings were taken from the store as they were
/// kept, 0 when they were made again.
pub(crate) fn crossed_after(edit: (&'static str, &'static str)) -> Result<(usize, usize), LintError> {
    let store = Arc::new(LintStoreFake::default());
    let given = |files: Vec<(&'static str, &'static str)>| Given {
        files,
        store: Arc::clone(&store),
        root: "root",
        query: Arc::new(QueryLanguageFake::counting_across_files()),
        language: LanguageFake {
            named_by_text: true,
            ..LanguageFake::default()
        },
        ..violations()
    };
    let crossed = || {
        store.kept.lock().map_or_else(
            |_| Vec::new(),
            |kept| {
                kept.findings
                    .iter()
                    .filter(|finding| finding.made == FindingMade::AcrossFiles)
                    .cloned()
                    .collect::<Vec<_>>()
            },
        )
    };
    let cold = lint(given(SOURCES.to_vec()));
    assert_eq!(cold.parses, 3, "a cold run reads every file");
    let before = crossed();
    assert!(
        !before.is_empty(),
        "the cold run keeps the findings of its cross-file steps"
    );
    let edited = lint(given(vec![edit, SOURCES[1], SOURCES[2]]));
    let _unread = edited.report?;
    Ok((edited.parses, usize::from(crossed() == before)))
}

/// A lint, then another over the same files and store: answers how many files the second asked the corpus for and how many the
/// language read.
pub(crate) fn unchanged() -> (usize, usize) {
    let store = Arc::new(LintStoreFake::default());
    let cold = again(&SOURCES, "root", "a rule", &store);
    let warm = again(&SOURCES, "root", "a rule", &store);
    assert_eq!(
        warm.rows, cold.rows,
        "an unchanged repository answers the same"
    );
    assert_eq!(
        store.keeps.load(Ordering::SeqCst),
        1,
        "and keeps nothing new: the store is as it was"
    );
    (warm.reads, warm.parses)
}

/// A lint, a lint over an edited file that fails while it writes its findings, then a lint over the files as they first were:
/// answers 1 when the store is as the first lint left it, and how many files the last lint read.
pub(crate) fn failed_run() -> (usize, usize) {
    let store = Arc::new(LintStoreFake::default());
    let snapshot = || {
        store
            .kept
            .lock()
            .map_or_else(|_| Stored::default(), |kept| kept.clone())
    };
    let _cold = again(&SOURCES, "root", "a rule", &store);
    let before = snapshot();
    let failed = lint(Given {
        files: vec![("a.rs", "fn changed() {}"), SOURCES[1], SOURCES[2]],
        store: Arc::clone(&store),
        root: "root",
        ends: Ends::Failed,
        ..violations()
    });
    assert!(
        matches!(failed.report, Err(LintError::Output { .. })),
        "the run fails writing its findings"
    );
    let after = snapshot();
    let warm = again(&SOURCES, "root", "a rule", &store);
    (usize::from(after == before), warm.parses)
}

/// An edited file is read again, and its findings are the ones a cold run gives.
pub(crate) fn edited() -> Result<(usize, usize), LintError> {
    let fresh = || Arc::new(LintStoreFake::default());
    let store = fresh();
    let cold = again(&SOURCES, "root", "a rule", &store);
    let edited = [("a.rs", "fn a() { 1 }"), SOURCES[1], SOURCES[2]];
    let edit = again(&edited, "root", "a rule", &store);
    assert_ne!(edit.rows, cold.rows, "its findings are new");
    assert_eq!(edit.rows.len(), cold.rows.len());
    let cold_edit = again(&edited, "other", "a rule", &fresh());
    assert_eq!(
        edit.rows, cold_edit.rows,
        "and the answer is the one a cold run gives"
    );
    read_and_unread(edit)
}

/// A removed file reports nothing, and the others report what they did.
pub(crate) fn deleted() -> Result<(usize, usize), LintError> {
    let store = Arc::new(LintStoreFake::default());
    let cold = again(&SOURCES, "root", "a rule", &store);
    let fewer = again(&SOURCES[..1], "root", "a rule", &store);
    assert_eq!(
        fewer.rows.len(),
        cold.rows.len() - 1,
        "and the one removed reports nothing"
    );
    assert!(
        fewer
            .rows
            .iter()
            .all(|row| row.0.iter().any(|cell| matches!(
                cell,
                Some(Typed::File(text) | Typed::Text(text)) if text.contains("a.rs")
            )))
    );
    read_and_unread(fewer)
}

/// Whether twenty runs in a row leave the store the size the fifth left it, as 1 or 0.
pub(crate) fn many_runs() -> Result<(usize, usize), Failure> {
    let store = Arc::new(LintStoreFake::default());
    let sizes = |held: &Arc<LintStoreFake>| {
        let kept = held
            .kept
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        (
            kept.rules.len(),
            kept.files.len(),
            kept.findings.len(),
            kept.unread.len(),
            kept.unknowns.len(),
        )
    };
    let mut after_five = (0, 0, 0, 0, 0);
    for round in 0_u32..20_u32 {
        let edited = if round % 2_u32 == 0_u32 {
            "fn a() {}"
        } else {
            "fn a() { }"
        };
        let files = [("a.rs", edited), SOURCES[1], SOURCES[2]];
        let _rows = again(&files, "root", "a rule", &store).report?;
        if round == 4_u32 {
            after_five = sizes(&store);
        }
    }
    Ok((usize::from(sizes(&store) == after_five), 0))
}

/// Where a finding is placed: its line and character, as read from its row.
pub(crate) fn placed() -> Result<(usize, usize), LintError> {
    let made = lint(violations());
    let first = made.rows.first().map(|row| &*row.0).unwrap_or_default();
    let (line, column) = match first {
        [
            Some(Typed::Text(_)),
            _,
            Some(Typed::Number(_)),
            ..,
            line,
            column,
        ] => (line, column),
        _ => (&None, &None),
    };
    assert!(
        first.len() >= 5,
        "a finding is its rule, subject, key count, key values and headed columns, then its line and character"
    );
    let at = |cell: &Option<Typed>| match cell {
        Some(Typed::Number(number)) => usize::try_from(number.0).unwrap_or_default(),
        Some(Typed::File(_) | Typed::Text(_) | Typed::Span(_)) | None => 0,
    };
    made.report.map(|_| (at(line), at(column)))
}

/// A plan that counts the files each function name is in and reads what the project writes: the plan of a rule that reads the project.
pub(crate) fn reading_the_project() -> QueryLanguageFake {
    let mut query = QueryLanguageFake::counting_across_files();
    query
        .plan
        .frontier
        .push(PlanInput::Relation(Relation::WorkspaceMember));
    query
}

/// A lint that counts what its steps are fed: how many times the language read the project, and how many rows the steps of the rules
/// were fed.
pub(crate) fn counted_lint(given: Given) -> Result<(usize, u64), Failure> {
    let asked = request(&given);
    let World {
        use_case, language, ..
    } = world(given);
    let report = use_case.counting().lint(asked)?;
    let fed = report
        .work
        .iter()
        .flat_map(|rule| &rule.steps)
        .map(|step| step.rows.0)
        .sum();
    Ok((language.projects.load(Ordering::SeqCst), fed))
}
