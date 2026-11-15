//! The lifecycle cases of a lint: unchanged, edited, deleted, failed and many runs over one store, and where a finding is placed.
use super::super::interface::LintUseCase as _;
type Failure = LintError;
use super::super::errors::LintError;
use super::given::Given;
use super::given::SOURCES;
use super::given::World;
use super::given::request;
use super::given::violations;
use super::worlds::again;
use super::worlds::compiled_lint;
use super::worlds::lint;
use super::worlds::read_and_unread;
use super::worlds::world;
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

/// Lints three files with a rule that counts the files each function name is in, edits `a.rs` to `edit`, and lints again over the
/// same store: answers how many files the second lint read, and 1 when the cross-file findings were taken from the store as they were
/// kept, 0 when they were made again.
pub(crate) fn crossed_after(
    edit: (&'static str, &'static str),
) -> Result<(usize, usize), LintError> {
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
    let report = use_case
        .counting(|| Arc::new(ryo_execution_runtime::outbound::observer::factory::counting()))
        .lint(asked)?;
    let fed = report
        .work
        .iter()
        .flat_map(|rule| &rule.steps)
        .map(|step| step.rows.0)
        .sum();
    Ok((language.projects.load(Ordering::SeqCst), fed))
}

/// Two counted lints of one use case over the same files, with two rules whose text differs and whose plan is one, so the second run
/// does the first's work again rather than reusing it: how many rows the steps of the rules were fed in the first run, and in the
/// second.
pub(crate) fn counted_twice() -> Result<(usize, usize), Failure> {
    let given = |rule| Given {
        rules: vec![rule],
        ..violations()
    };
    let (first_asked, second_asked) = (request(&given("one")), request(&given("two")));
    let World { use_case, .. } = world(given("one"));
    let counted = use_case
        .counting(|| Arc::new(ryo_execution_runtime::outbound::observer::factory::counting()));
    let fed = |report: super::super::types::LintReport| {
        let rows: u64 = report
            .work
            .iter()
            .flat_map(|rule| &rule.steps)
            .map(|step| step.rows.0)
            .sum();
        usize::try_from(rows).expect("a test's row count fits a usize")
    };
    let first = fed(counted.lint(first_asked)?);
    let second = fed(counted.lint(second_asked)?);
    Ok((first, second))
}

/// Lints [`a.rs`, `b.rs`] with two rules over one store: `files` governs both files and reads `filesystem.file`; `exec` governs
/// `a.rs` alone and reads `filesystem.executable`, which every file of the fake corpus has read in part (its permission is
/// unknown). Then the configuration turns `exec` on for `b.rs` (its include widens to every file) and the lint runs again over
/// the same store, `b.rs` unchanged. A file's run asks its sources only for the relations of the rules that run on it, so the
/// first run's record of `b.rs` holds no part read for `exec`; the config change must not replay that record. Answers 1 when
/// the warm run reads `b.rs` again and answers what a cold run of the new configuration answers, `exec`'s part of `b.rs`
/// among it.
pub(crate) fn a_rule_turned_on_for_a_kept_file_reads_it_again_with_its_parts()
-> Result<(usize, usize), LintError> {
    const FILES: &str = "let files = capture(from=\"filesystem.file\", path=\"path\", pattern=\"{name}.rs\", captures=[\"name\"]);\nemit files subject \"path\" key(\"path\");";
    const EXEC: &str = "let exec = capture(from=\"filesystem.executable\", path=\"path\", pattern=\"{name}.rs\", captures=[\"name\"]);\nemit exec subject \"path\" key(\"path\");";
    let given = |store: Arc<LintStoreFake>| Given {
        files: vec![("a.rs", "fn a() {}"), ("b.rs", "fn b() {}")],
        rules: vec![FILES, EXEC],
        ids: vec!["files", "exec"],
        include: vec!["**"],
        store,
        ..violations()
    };
    // The rule `exec`, governing `a.rs` alone or, after the change, every file.
    let asked = |given: &Given, exec: &str| {
        let mut asked = request(given);
        if let Some(rule) = asked.rules.get_mut(1) {
            rule.include = vec![ryo_values::domain::files::types::Glob(exec.into())];
        }
        asked
    };
    let linted = |given: Given, exec: &str| -> Result<(String, String, usize), LintError> {
        let request = asked(&given, exec);
        let (world, report) = compiled_lint(given, request)?;
        let rows = std::mem::take(
            &mut world
                .collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .rows,
        );
        Ok((
            format!("{rows:?}"),
            format!("{:?}", report.unknowns),
            world.language.parses.load(Ordering::SeqCst),
        ))
    };
    let store = Arc::new(LintStoreFake::default());
    let before = linted(given(Arc::clone(&store)), "a.rs")?;
    assert!(
        !before.1.contains("b.rs"),
        "before the change no rule reading the executable bit governs `b.rs`: {}",
        before.1
    );
    let changed = linted(given(Arc::clone(&store)), "**")?;
    let cold = linted(given(Arc::new(LintStoreFake::default())), "**")?;
    assert!(
        cold.1.contains("exec") && cold.1.contains("b.rs"),
        "a cold run of the new configuration names `exec`'s part of `b.rs`: the case has a part to place: {}",
        cold.1
    );
    assert_eq!(
        changed.2, 2,
        "the config change reads both files again: a kept record of `b.rs` is not replayed"
    );
    assert_eq!(
        (&changed.0, &changed.1),
        (&cold.0, &cold.1),
        "the run after the change answers what a cold run of the new configuration answers"
    );
    Ok((usize::from(changed.1 == cold.1), 0))
}
