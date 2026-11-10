//! The cases of every store answering alike, of syntax applicability replayed, of project entries kept, and of a part read in
//! part kept as its file's own.
use super::super::errors::LintError;
use super::super::interface::LintUseCase as _;
use super::given::Given;
use super::given::SOURCES;
use super::given::World;
use super::given::request;
use super::lifecycle::counted_lint;
use super::lifecycle::reading_the_project;
use super::worlds::scheduled_world_with_query;
use super::worlds::world;
use crate::inbound::use_cases::lint::fixtures::given::violations;
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::row_sink;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::WorkerCount;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_execution_runtime::outbound::observer::factory::counting;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::runs::types::AnswerShape;
use std::sync::Arc;
use std::sync::atomic::Ordering;

/// What one whole lint answered: the rows written, what the rules could not decide, the files a rule skipped, and how many files the
/// language parsed.
pub(crate) struct Answered {
    pub(crate) rows: Vec<row_sink::types::KeptRow>,
    pub(crate) undecided: String,
    pub(crate) skipped: String,
    pub(crate) parses: usize,
}

/// A lint of `files` with the cross-file rule, over `kept_in`, when the language rejects the syntax of `rejected`.
pub(crate) fn answered(
    files: &[(&'static str, &'static str)],
    rejected: &[&str],
    applicability: ryo_values::domain::runs::types::RuleApplicability,
    answer: AnswerShape,
    kept_in: Arc<dyn crate::outbound::lint_store::interface::LintStorePort>,
) -> Answered {
    let mut given = Given {
        files: files.to_vec(),
        query: Arc::new(QueryLanguageFake::counting_across_files()),
        answer,
        kept_in: Some(kept_in),
        ..violations()
    };
    given.language.rejected_syntax = rejected
        .iter()
        .map(|path| FilePath((*path).into()))
        .collect();
    let mut asked = request(&given);
    asked.rules[0].applicability = applicability;
    let world = world(given);
    let report = world.use_case.lint(asked).expect("the lint answers");
    Answered {
        rows: std::mem::take(
            &mut *world
                .collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
        .rows,
        undecided: format!("{:?}", report.unknowns),
        skipped: format!("{:?}", report.skipped),
        parses: world.language.parses.load(Ordering::SeqCst),
    }
}

/// A store of the kind `kind` names, kept under `directory` when it is kept on disk.
pub(crate) fn store_of(
    kind: &str,
    directory: &std::path::Path,
) -> Arc<dyn crate::outbound::lint_store::interface::LintStorePort> {
    match kind {
        "memory" => Arc::new(crate::outbound::lint_store::factory::memory()),
        "postcard" => Arc::new(crate::outbound::lint_store::factory::postcard(
            crate::outbound::lint_store::types::Store {
                directory: crate::outbound::lint_store::types::CacheDirectory(
                    directory.to_string_lossy().into(),
                ),
                build: crate::outbound::lint_store::types::BuildIdentity("reuse".into()),
            },
        )),
        _ => Arc::new(crate::outbound::lint_store::factory::discard()),
    }
}

pub(crate) const THREE: [(&str, &str); 3] = [
    ("a.rs", "fn a() {}"),
    ("b.rs", "fn b() {}"),
    ("c.rs", "fn c() {}"),
];

pub(crate) fn accepted_syntax_applicability_replays_skips_and_invalidates_on_policy_change()
-> Result<(usize, usize), LintError> {
    use ryo_values::domain::runs::types::{RuleApplicability, SkipReason};

    let files = vec![("a.rs", "fn a() {}"), ("b.rs", "fn b() {}")];
    let store = Arc::new(LintStoreFake::default());
    let mut given = Given {
        files: files.clone(),
        store: Arc::clone(&store),
        ..violations()
    };
    given.language.rejected_syntax = vec![FilePath("b.rs".into())];
    let mut asked = request(&given);
    asked.rules[0].applicability = RuleApplicability::AcceptedSyntax;
    let first_world = world(given);

    let first = first_world
        .use_case
        .lint(asked.clone())
        .expect("first lint");
    assert_eq!(first.unknowns, []);
    assert_eq!(first.skipped.len(), 1);
    assert_eq!(first.skipped[0].path, FilePath("b.rs".into()));
    assert_eq!(first.skipped[0].reason, SkipReason::SyntaxRejected);
    assert_eq!(first_world.language.parses.load(Ordering::SeqCst), 2);
    let first_rows = first_world
        .collected
        .0
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .rows
        .clone();
    assert_eq!(
        first_rows.len(),
        1,
        "the rejected file does not run the rule"
    );

    let warm = first_world.use_case.lint(asked.clone()).expect("warm lint");
    assert_eq!(warm.skipped, first.skipped, "the stored skip is replayed");
    assert_eq!(first_world.language.parses.load(Ordering::SeqCst), 2);

    let edited = vec![("a.rs", "fn changed() {}"), files[1]];
    let mut edited_given = Given {
        files: edited,
        store: Arc::clone(&store),
        ..violations()
    };
    edited_given.language.rejected_syntax = vec![FilePath("b.rs".into())];
    let edited_world = world(edited_given);
    let edited_report = edited_world
        .use_case
        .lint(asked.clone())
        .expect("edited lint");
    assert_eq!(edited_report.skipped, first.skipped);
    assert_eq!(edited_world.language.parses.load(Ordering::SeqCst), 1);
    assert_eq!(
        edited_world
            .collected
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .rows
            .len(),
        1,
        "the edited accepted file is evaluated and the rejected file stays skipped"
    );

    asked.rules[0].applicability = RuleApplicability::Always;
    let mut policy_given = Given {
        files,
        store,
        ..violations()
    };
    policy_given.language.rejected_syntax = vec![FilePath("b.rs".into())];
    let policy_world = world(policy_given);
    let policy_report = policy_world
        .use_case
        .lint(asked)
        .expect("always-applicable lint");
    assert!(policy_report.skipped.is_empty());
    assert_eq!(
        policy_world
            .collected
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .rows
            .len(),
        2,
        "changing the rule policy invalidates the retained skip"
    );
    Ok((1, 0))
}

pub(crate) fn project_entry_observations_invalidate_persisted_absence_and_presence()
-> Result<(usize, usize), LintError> {
    let store = Arc::new(LintStoreFake::default());
    let entry = ("../pkg.py", "package = 1");
    let given = |outside: Vec<(&'static str, &'static str)>| Given {
        files: vec![
            SOURCES[0],
            SOURCES[1],
            ("project.manifest.fake", "members = 1"),
        ],
        outside,
        store: Arc::clone(&store),
        root: "root",
        query: Arc::new(reading_the_project()),
        language: LanguageFake {
            project_entry_paths: vec![FilePath(entry.0.into())],
            ..LanguageFake::default()
        },
        ..violations()
    };
    for (outside, expected_project_reads) in [
        (Vec::new(), 1),
        (Vec::new(), 0),
        (vec![entry], 1),
        (vec![entry], 0),
        (Vec::new(), 1),
    ] {
        let (project_reads, _fed) = counted_lint(given(outside)).expect("lint");
        assert_eq!(project_reads, expected_project_reads);
    }
    Ok((1, 0))
}

pub(crate) fn project_reuse_frames_root_and_repository_and_keeps_dependency_evidence()
-> Result<(usize, usize), LintError> {
    use ryo_language::inbound::use_cases::project::types::Repository;
    let store = Arc::new(LintStoreFake::default());
    let invoke = |root: &'static str, repository, project| {
        let given = Given {
            root,
            store: Arc::clone(&store),
            files: vec![SOURCES[0], SOURCES[1]],
            outside: vec![("../context.fake", "content")],
            query: Arc::new(if project {
                reading_the_project()
            } else {
                QueryLanguageFake::counting_across_files()
            }),
            language: LanguageFake {
                consulted: if project {
                    vec![
                        FilePath("../context.fake".into()),
                        FilePath("../absent.fake".into()),
                    ]
                } else {
                    vec![]
                },
                ..LanguageFake::default()
            },
            ..violations()
        };
        let mut asked = request(&given);
        asked.session.repository = repository;
        let made = world(given);
        let counted = made.use_case.counting(|| Arc::new(counting()));
        let report = counted.lint(asked).expect("complete lint");
        (made.language.projects.load(Ordering::SeqCst), report)
    };
    let (cold, cold_report) = invoke("root-a", Repository::Disk, true);
    let (warm, warm_report) = invoke("root-a", Repository::Disk, true);
    assert_eq!(
        (cold, warm),
        (1, 0),
        "an unchanged project really reuses its kept rows"
    );
    assert_eq!(cold_report.dependencies, warm_report.dependencies);
    assert_eq!(
        warm_report.dependencies.len(),
        2,
        "present and missing dependencies survive reuse"
    );
    assert_eq!(
        invoke("root-b", Repository::Disk, true).0,
        1,
        "another absolute root cannot reuse package context"
    );
    assert_eq!(
        invoke("root-b", Repository::Given, true).0,
        1,
        "repository isolation is part of project resolution identity"
    );
    assert_eq!(invoke("root-b", Repository::Given, true).0, 0);
    let _cold = invoke("root-a", Repository::Disk, false);
    let (projects, nonproject) = invoke("root-b", Repository::Given, false);
    assert_eq!(projects, 0);
    assert_eq!(
        nonproject
            .work
            .iter()
            .flat_map(|rule| &rule.steps)
            .map(|step| step.rows.0)
            .sum::<u64>(),
        0,
        "nonproject cache identity is unchanged by root and mode"
    );
    Ok((1, 0))
}

/// Lints [`a.rs`, `b.rs`] with a cross-file rule that trusts the absence of `filesystem.executable` rows, which every file of the
/// fake corpus has read in part (its permission is unknown); lists `a2.rs` before `b.rs`; and lints again over the same store with
/// `b.rs` unchanged, so `b.rs` is used again at another number. Answers 1 when the edit run answers and keeps what a cold run of the
/// three files answers and keeps: the part of `b.rs` read in part is `b.rs`'s, never the file now at its old number.
pub(crate) fn partial_of_a_reused_file_after_a_file_is_listed_before_it()
-> Result<(usize, usize), LintError> {
    const PLAIN: &str = "let executables = capture(from=\"filesystem.executable\", path=\"path\", pattern=\"{name}.rs\", captures=[\"name\"]);\nlet named = project(from=executables, columns=[\"name\"]);\nlet files = capture(from=\"filesystem.file\", path=\"path\", pattern=\"{name}.rs\", captures=[\"name\"]);\nlet plain = antijoin(left=files, right=named, left-on=[\"name\"], right-on=[\"name\"]);\nemit plain subject \"path\" key(\"path\");";
    let given = |files: Vec<(&'static str, &'static str)>, store: Arc<LintStoreFake>| Given {
        files,
        rules: vec![PLAIN],
        ids: vec!["plain"],
        store,
        ..violations()
    };
    let linted = |given: Given| -> Result<(Vec<row_sink::types::KeptRow>, String), LintError> {
        let (world, report) = compiled_lint(given)?;
        let rows = std::mem::take(
            &mut world
                .collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .rows,
        );
        Ok((rows, format!("{:?}", report.unknowns)))
    };
    let kept = |store: &LintStoreFake| {
        let kept = store
            .kept
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let b = kept
            .files
            .iter()
            .position(|file| &*file.path.0 == "b.rs")
            .and_then(|place| u32::try_from(place).ok());
        (b, kept.handed.covered.clone())
    };
    let (a, b, a2) = (
        ("a.rs", "fn a() {}"),
        ("b.rs", "fn b() {}"),
        ("a2.rs", "fn a2() {}"),
    );
    let store = Arc::new(LintStoreFake::default());
    let _cold = linted(given(vec![a, b], Arc::clone(&store)))?;
    let edit = linted(given(vec![a, a2, b], Arc::clone(&store)))?;
    let fresh = Arc::new(LintStoreFake::default());
    let oracle = linted(given(vec![a, a2, b], Arc::clone(&fresh)))?;
    assert_eq!(
        edit, oracle,
        "the edit run answers what a cold run of the three files answers"
    );
    let (b_now, covered) = kept(&store);
    let (b_fresh, fresh_covered) = kept(&fresh);
    assert!(
        b_fresh.is_some_and(|b| fresh_covered
            .iter()
            .any(|row| row.file.0 == b && !row.partials.is_empty())),
        "the cold run keeps `b.rs` read in part: the case has a part to place"
    );
    assert_eq!(b_now, b_fresh, "both stores number `b.rs` alike");
    assert_eq!(
        covered, fresh_covered,
        "the part of `b.rs` the edit run keeps is the one a cold run keeps"
    );
    Ok((usize::from(covered == fresh_covered), 0))
}

/// One lint of `given` with the real query compiler: the world, for its collected rows, and the report.
pub(crate) fn compiled_lint(
    given: Given,
) -> Result<(World, crate::inbound::use_cases::lint::types::LintReport), LintError> {
    use ryo_execution::outbound::query_language::factory;
    let syntax = || Arc::new(ryo_query_language::outbound::syntax::factory::winnow());
    let catalog = || Arc::new(ryo_query_language::outbound::catalog::factory::language());
    let compile = Arc::new(
        ryo_query_language::inbound::use_cases::compile::use_case::CompileUseCaseImpl::new(
            syntax(),
            catalog(),
        ),
    );
    let typecheck = Arc::new(
        ryo_query_language::inbound::use_cases::typecheck::use_case::TypecheckUseCaseImpl::new(
            syntax(),
            catalog(),
        ),
    );
    let asked = request(&given);
    let scheduler: Arc<dyn SchedulerPort> = Arc::new(SchedulerFake::default());
    let world = scheduled_world_with_query(
        given,
        scheduler,
        WorkerCount(1),
        Arc::new(factory::use_case(compile, typecheck)),
    );
    let report = world.use_case.lint(asked)?;
    Ok((world, report))
}
