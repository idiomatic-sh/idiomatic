// the 37 rows each drive the whole lint use case through fakes (warm, edited, deleted and changed-rule runs): a row is one whole run, and its expected rows are the full answer of that run.
//! What the `lint` use case promises, against the fakes of the ports it calls.

use crate::outbound::rows;
use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::errors::LintError;
use super::fixtures::contract::check;
use super::interface::LintUseCase as _;
use super::types::{LintRequest, Rule};
use super::use_case::LintUseCaseImpl;
use crate::cases::{run, selects, Case, CaseName, Expect, Word};
use crate::outbound::corpus::fixtures::fake::CorpusFake;
use crate::outbound::digest::fixtures::fake::DigestFake;
use crate::outbound::facts::fixtures::fake::FactsFake;
use crate::outbound::language::fixtures::fake::{FakeCatalog, LanguageFake};
use crate::outbound::matching::fixtures::fake::MatchingFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::query_language::fixtures::fake::{QueryLanguageFake, REFUSED};
use crate::outbound::rows::fixtures::fake::{Ends, RowsFake};
use crate::outbound::rows::interface::RowsPort;
use ryo_values::domain::runs::types::AnswerShape;
use ryo_values::domain::runs::types::RuleId;

use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use crate::outbound::scheduler::interface::SchedulerPort;
use crate::outbound::scheduler::types::WorkerCount;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::files::types::{CorpusFile, FileBytes, Glob};
use ryo_values::domain::hosted::types::{
    HostColumn, HostColumnName, HostKind, HostName, HostRelation, HostRow,
};
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, HostedNumber, Input as PlanInput, LiteralText, Operand, ParameterName,
    QueryLiteral, QueryText, StepId, Test,
};
use ryo_values::domain::plans::types::{
    Emit, Operation, ParameterType, Plan, PlanParameter, PlanStep,
};
use ryo_values::domain::records::types::{Cell as Typed, RowTotal};
use ryo_values::domain::runs::types::{Budget, Session};
use ryo_values::domain::stores::types::{FindingMade, Stored};

/// What a row gives the use case: the ports it calls, the rules and the files linted.
struct Given {
    outside_executable: ryo_values::domain::files::types::ExecutablePermission,
    files: Vec<(&'static str, &'static str)>,
    /// Files on the disk the corpus does not list, such as those above the root.
    outside: Vec<(&'static str, &'static str)>,
    corpus_fails: bool,
    facts: FactsFake,
    language: LanguageFake,
    hosted: Vec<HostRelation>,
    memory: MemoryFake,
    query: Arc<QueryLanguageFake>,
    scheduler: SchedulerFake,
    ends: Ends,
    store: Arc<LintStoreFake>,
    /// A store of another kind the run keeps in instead of `store`, when a row is about the stores themselves.
    kept_in: Option<Arc<dyn crate::outbound::lint_store::interface::LintStorePort>>,
    root: &'static str,
    rules: Vec<&'static str>,
    ids: Vec<&'static str>,
    only: Vec<&'static str>,
    include: Vec<&'static str>,
    exclude: Vec<&'static str>,
    answer: AnswerShape,
}

/// What a lint over a given made: the answer, the rows kept, the files the language read and what the ports were asked.
struct Lint {
    report: Result<usize, LintError>,
    rows: Vec<rows::types::Row>,
    parses: usize,
    projects: usize,
    compiles: usize,
    /// How many files the corpus was asked to read.
    reads: usize,
}

/// What a row of the cases asks of the use case.
#[derive(Clone, Copy)]
enum Scenario {
    /// One lint over the fakes the function builds.
    Lint(fn() -> Given),
    /// Lints the same files twice over one store.
    Warm,
    /// Lints the files again after one of them was edited.
    Edited,
    /// Lints the files again after two of them were deleted.
    Deleted,
    /// Lints the files again with another rule.
    ChangedRule,
    /// Lints with a store that cannot be written or read.
    BrokenStore(fn() -> LintStoreFake),
    /// Lints twice with two rules, with one use case.
    CompilesPerSession,
    /// Lints once with two rules.
    CompilesPerRule,
    /// Lints many times in a row over one store, a file edited back and forth: what the store keeps stops growing.
    ManyRuns,
    /// Lints a rule, then the rule of the same id with another query, over plans one session shares.
    ChangedQueryKeepsOnePlan,
    /// Changes a directly imported module with the real compiler and checks changed findings.
    ChangedFragmentPlan,
    /// Changes only a nested query module and checks both plan and persisted-result invalidation.
    ChangedNestedModule,
    /// Two runs naming different rules, one after the other twice over plans one session shares, as concurrent `tm validate` calls do.
    InterleavedRuns,
    /// Lints with the host's relation, counting how many times the language was asked to read the manifests.
    Projects,
    /// Lints and reads the line and the character the first finding is at.
    Placed,
    /// The pipeline's lifecycle: a lint, then another over the same files and store.
    Unchanged,
    /// The pipeline's lifecycle: a lint with a rule that crosses files, then one after a file's body is edited and what it hands
    /// over is as it was.
    BodyEdited,
    /// The same, after the function a file declares is renamed: the changed file is read on its own, and the cross-file steps are
    /// made again from what the other files handed the last run.
    ExportRenamed,
    /// The pipeline's lifecycle: a lint, a lint that fails while it writes its findings, then a lint over the same files.
    FailedRun,
    /// Lints twice a file whose bytes cannot be read alike twice: the second lint reads it again.
    UnreadableAgain,
    /// Lints a rule that reads the project twice over one store, with what changed between the two.
    Consulted(Between),
}

/// What changes between two lints of a rule that reads the project, whose lookup reads a configuration above the root.
#[derive(Clone, Copy)]
enum Between {
    /// Nothing.
    Nothing,
    /// The configuration above the root.
    ConfigurationAbove,
    /// The manifest the run lists.
    Manifest,
    /// The configuration above the root is gone.
    ConfigurationGone,
    /// A configuration the lookup looked for above the root and found none of is there now.
    ConfigurationAppears,
    /// The lookup found the older configuration above the root, and the newer one that comes before it is there now.
    OlderBecomesNewer,
}

/// What a row of the cases asks of the use case.
type Input = Scenario;
/// What a scenario answers: how many findings or files were read, and how many files were left unread.
type Value = (usize, usize);
type Failure = LintError;

const SOURCES: [(&str, &str); 3] = [
    ("a.rs", "fn a() {}"),
    ("b.rs", "fn b() {}"),
    ("notes.txt", "notes"),
];

/// A plan over `function`: every row, or only the rows whose name is the text given.
fn plan_with_name(name: &str) -> Plan {
    let comparison = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(name.into()))),
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let mut plan = QueryLanguageFake::selecting().plan;
    if let Some(PlanStep {
        operation: Operation::Select { comparisons, .. },
        ..
    }) = plan.steps.first_mut()
    {
        comparisons.push(comparison);
    }
    plan
}

/// A plan that reads the rows of the one relation the host declared: they are made once, after the files, and each is a finding.
fn plan_over_hosted() -> Plan {
    let mut plan = plan_with_name("");
    plan.steps = vec![
        PlanStep {
            operation: Operation::Hosted {
                relation: HostedNumber(0),
            },
            scope: StepScope::CrossFile,
        },
        PlanStep {
            operation: Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
            scope: StepScope::CrossFile,
        },
    ];
    plan.emit = Emit {
        step: StepId(1),
        subject: ColumnIndex(0),
        key: vec![ColumnIndex(1)],
        headings: Vec::new(),
    };
    plan
}

/// The host's relation: the files it places, and what the rule reads of them.
fn host_placed() -> HostRelation {
    let row = |path: &str| HostRow(vec![Typed::File(path.into()), Typed::Text("docs".into())]);
    let column = |name: &str, kind| HostColumn {
        name: HostColumnName(name.into()),
        kind,
    };
    HostRelation {
        name: HostName("host.placed".into()),
        columns: vec![
            column("path", HostKind::File),
            column("spec", HostKind::Text),
        ],
        rows: vec![row("notes.txt"), row("not-there.txt")],
    }
}

/// Every function of `a.rs` and `b.rs` is a violation, `notes.txt` is read by no language.
fn violations() -> Given {
    Given {
        outside_executable: Default::default(),
        files: SOURCES.to_vec(),
        outside: Vec::new(),
        corpus_fails: false,
        facts: FactsFake::default(),
        language: LanguageFake::default(),
        hosted: Vec::new(),
        memory: MemoryFake::default(),
        query: Arc::new(QueryLanguageFake::selecting()),
        scheduler: SchedulerFake::default(),
        ends: Ends::Well,
        store: Arc::new(LintStoreFake::default()),
        kept_in: None,
        root: ".",
        rules: vec!["a rule"],
        ids: Vec::new(),
        only: Vec::new(),
        include: Vec::new(),
        exclude: Vec::new(),
        answer: AnswerShape::Findings,
    }
}

/// The use case and the fakes whose counts a row reads.
struct World {
    use_case: LintUseCaseImpl,
    collected: rows::types::Collector,
    language: Arc<LanguageFake>,
    query: Arc<QueryLanguageFake>,
    reads: Arc<std::sync::atomic::AtomicUsize>,
}

/// The request to lint what `given` stands for.
fn request(given: &Given) -> LintRequest {
    let globs = |texts: &[&str]| texts.iter().map(|glob| Glob((*glob).into())).collect();
    LintRequest {
        source_language: None,
        answer: given.answer,
        hosted: given.hosted.clone(),
        session: Session {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(given.root.into()),
        },
        files: given
            .only
            .iter()
            .map(|path| FilePath((*path).into()))
            .collect(),
        rules: given
            .rules
            .iter()
            .enumerate()
            .map(|(place, query)| Rule {
                observations: Vec::new(),
                id: RuleId(
                    given
                        .ids
                        .get(place)
                        .map_or_else(|| place.to_string().into(), |id| (*id).into()),
                ),
                query: ryo_values::domain::program::types::QueryBundle::from(QueryText(
                    (*query).into(),
                )),
                applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
                parameters: Vec::new(),
                include: globs(&given.include),
                exclude: globs(&given.exclude),
            })
            .collect(),
    }
}

#[test]
fn accepted_syntax_applicability_replays_skips_and_invalidates_on_policy_change() {
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
}

/// The use case over the fakes `given` stands for.
fn world(mut given: Given) -> World {
    let scheduler = Arc::new(std::mem::take(&mut given.scheduler));
    let query_language: Arc<
        dyn ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort,
    > = Arc::<QueryLanguageFake>::clone(&given.query);
    scheduled_world_with_query(given, scheduler, WorkerCount(1), query_language)
}

/// The same fixture over an explicitly injected production scheduler.
fn scheduled_world(given: Given, scheduler: Arc<dyn SchedulerPort>, workers: WorkerCount) -> World {
    let query_language: Arc<
        dyn ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort,
    > = Arc::<QueryLanguageFake>::clone(&given.query);
    scheduled_world_with_query(given, scheduler, workers, query_language)
}

/// The same test world with an explicitly supplied query compiler.
fn scheduled_world_with_query(
    given: Given,
    scheduler: Arc<dyn SchedulerPort>,
    workers: WorkerCount,
    query_language: Arc<
        dyn ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort,
    >,
) -> World {
    let memory = Arc::new(given.memory);
    let operators = Arc::new(crate::outbound::operators::factory::use_case(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
    ));
    scheduled_world_with_query_resources(
        given,
        scheduler,
        workers,
        query_language,
        memory,
        operators,
    )
}

fn scheduled_world_with_query_resources(
    given: Given,
    scheduler: Arc<dyn SchedulerPort>,
    workers: WorkerCount,
    query_language: Arc<
        dyn ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort,
    >,
    analysis_memory: Arc<dyn crate::outbound::memory::interface::MemoryPort>,
    operators: Arc<dyn ryo_execution::inbound::use_cases::execution::types::OperatorsPort>,
) -> World {
    let collected = rows::types::Collector::default();
    let kept: Arc<dyn RowsPort> = Arc::new(rows::factory::collect(collected.clone()));
    let ports: Arc<dyn RowsPort> = if given.ends == Ends::Well {
        kept
    } else {
        Arc::new(RowsFake {
            ends: given.ends,
            ..RowsFake::default()
        })
    };
    let language = Arc::new(given.language);
    let reads = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let corpus_file = |(path, text): &(&str, &str)| CorpusFile {
        executable: Default::default(),
        path: FilePath((*path).into()),
        bytes: FileBytes(text.as_bytes().to_vec()),
    };
    let files = given.files.iter().map(corpus_file);
    let outside = given
        .outside
        .iter()
        .map(corpus_file)
        .map(|mut file| {
            file.executable = given.outside_executable;
            file
        })
        .collect();
    let store: Arc<dyn crate::outbound::lint_store::interface::LintStorePort> = match given.kept_in
    {
        Some(other) => other,
        None => given.store,
    };
    let use_case = {
        let analysis_corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort> =
            Arc::new(CorpusFake {
                files: files.collect(),
                fails: given.corpus_fails,
                reads: Arc::clone(&reads),
                outside,
            });
        let analysis_language: Arc<dyn crate::outbound::language::interface::LanguagePort> =
            Arc::<LanguageFake>::clone(&language);
        let analysis_digest: Arc<
            dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort,
        > = Arc::new(DigestFake);
        LintUseCaseImpl::new(
            crate::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            crate::application::cross_file::service::CrossFile::new(Arc::clone(&analysis_corpus), std::sync::Arc::new(crate::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())))),
            Arc::clone(&analysis_corpus),
            Arc::new(given.facts),
            Arc::clone(&analysis_language),
            Arc::new(MatchingFake),
            query_language,
            Arc::clone(&analysis_memory),
            operators,
            std::sync::Arc::new(crate::outbound::derived::factory::use_case(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            )),
            scheduler,
            workers,
            ports,
            Arc::clone(&analysis_digest),
            store,
        )
    };
    World {
        use_case,
        collected,
        language,
        query: given.query,
        reads,
    }
}

/// One lint over what `given` stands for.
fn lint(given: Given) -> Lint {
    let asked = request(&given);
    let world = world(given);
    let report = world.use_case.lint(asked);
    Lint {
        report: report.map(|answer| answer.unread.len()),
        rows: std::mem::take(
            &mut *world
                .collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
        .rows,
        parses: world.language.parses.load(Ordering::SeqCst),
        projects: world.language.projects.load(Ordering::SeqCst),
        compiles: world.query.calls.load(Ordering::SeqCst),
        reads: world.reads.load(Ordering::SeqCst),
    }
}

/// Complete ordered answers agree across worker counts, including reused and edited stores.
fn serial_and_parallel_agree() {
    let fresh = || Arc::new(LintStoreFake::default());
    let serial = fresh();
    let parallel = fresh();
    let edited = [("a.rs", "fn changed() {}"), SOURCES[1], SOURCES[2]];
    for files in [&SOURCES, &SOURCES, &edited] {
        compare_schedulers(files, &serial, &parallel, false, Ends::Well);
    }
    compare_schedulers(&SOURCES, &fresh(), &fresh(), true, Ends::Well);
    compare_schedulers(&SOURCES, &fresh(), &fresh(), false, Ends::Failed);
}

/// Compare every report field and the complete ordered output, rather than row counts.
fn compare_schedulers(
    files: &[(&'static str, &'static str)],
    serial: &Arc<LintStoreFake>,
    parallel: &Arc<LintStoreFake>,
    corpus_fails: bool,
    ends: Ends,
) {
    let execute =
        |store: &Arc<LintStoreFake>, scheduler: Arc<dyn SchedulerPort>, workers: WorkerCount| {
            let given = Given {
                files: files.to_vec(),
                store: Arc::clone(store),
                corpus_fails,
                ends,
                ..violations()
            };
            let asked = request(&given);
            let world = scheduled_world(given, scheduler, workers);
            (
                world.use_case.lint(asked),
                std::mem::take(
                    &mut *world
                        .collected
                        .0
                        .lock()
                        .unwrap_or_else(std::sync::PoisonError::into_inner),
                ),
            )
        };
    assert_eq!(
        execute(
            serial,
            Arc::new(crate::outbound::scheduler::factory::sequential()),
            WorkerCount(1)
        ),
        execute(
            parallel,
            Arc::new(crate::outbound::scheduler::factory::rayon(
                crate::outbound::scheduler::factory::pool(
                    ryo_values::domain::runs::types::WorkersAsked(2)
                )
            )),
            WorkerCount(2)
        ),
        "worker scheduling preserves every report field, column, row and output summary"
    );
}

/// The files read and the files left unread by a lint.
fn read_and_unread(run: Lint) -> Result<Value, Failure> {
    run.report.map(|unread| (run.parses, unread))
}

/// A lint of `files` at `root` with `rule` as the rule's text, over `store`.
fn again(
    files: &[(&'static str, &'static str)],
    root: &'static str,
    rule: &'static str,
    store: &Arc<LintStoreFake>,
) -> Lint {
    lint(Given {
        files: files.to_vec(),
        store: Arc::clone(store),
        root,
        rules: vec![rule],
        ..violations()
    })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("violation_is_reported_with_path_and_span"),
        word: Word::Succeeds,
        input: || Scenario::Lint(violations),
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("finding_carries_its_line_and_character"),
        word: Word::Succeeds,
        input: || Scenario::Placed,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("unread_files_are_reported"),
        word: Word::Succeeds,
        input: || Scenario::Lint(violations),
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("unsupported_file_is_unread_without_a_failure"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                files: vec![SOURCES[2]],
                ..violations()
            })
        },
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("language_project_is_not_called_when_no_rule_reads_what_it_writes"),
        word: Word::Succeeds,
        input: || Scenario::Projects,
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("clean_repository_has_no_findings"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                query: Arc::new(QueryLanguageFake {
                    plan: plan_with_name("nothing"),
                    ..QueryLanguageFake::default()
                }),
                files: SOURCES[..2].to_vec(),
                ..violations()
            })
        },
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("lint_is_scoped_to_the_files_given"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                only: vec!["a.rs"],
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_rule_governs_only_the_files_it_includes_and_the_others_are_not_read"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                include: vec!["a.rs"],
                ..violations()
            })
        },
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_rule_does_not_govern_the_files_it_leaves_out"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                exclude: vec!["**/*.txt"],
                ..violations()
            })
        },
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("language_parse_failure_is_unread_not_empty"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                files: vec![SOURCES[0], ("broken.rs", "fn main() {}")],
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("the_rows_of_a_relation_the_host_declared_are_read_by_a_rule"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                hosted: vec![host_placed()],
                files: vec![SOURCES[0], SOURCES[2]],
                query: Arc::new(QueryLanguageFake {
                    plan: plan_over_hosted(),
                    ..QueryLanguageFake::default()
                }),
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_file_that_changes_while_it_is_read_is_left_unread_and_not_kept"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                files: vec![SOURCES[0], ("unstable.rs", "fn main() {}")],
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("unchanged_files_are_not_revisited"),
        word: Word::Succeeds,
        input: || Scenario::Warm,
        expect: Expect::Value((0, 1)),
    },
    Case {
        name: CaseName("changed_file_is_visited_again"),
        word: Word::Succeeds,
        input: || Scenario::Edited,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("deleted_file_drops_its_findings"),
        word: Word::Succeeds,
        input: || Scenario::Deleted,
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("changed_rule_is_evaluated_again"),
        word: Word::Succeeds,
        input: || Scenario::ChangedRule,
        expect: Expect::Value((3, 1)),
    },
    Case {
        name: CaseName(
            "a_warm_run_with_nothing_changed_reads_no_project_and_makes_no_project_rule_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::Nothing),
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_changed_configuration_above_the_root_makes_the_project_rules_again"),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::ConfigurationAbove),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_changed_manifest_makes_the_project_rules_again"),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::Manifest),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName(
            "a_configuration_the_lookup_read_that_is_gone_makes_the_project_rules_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::ConfigurationGone),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName(
            "a_configuration_the_lookup_found_none_of_and_is_there_now_makes_the_project_rules_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::ConfigurationAppears),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName(
            "a_newer_configuration_before_the_older_one_the_lookup_found_makes_the_project_rules_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::Consulted(Between::OlderBecomesNewer),
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("compiles_each_rule_once_per_session"),
        word: Word::Succeeds,
        input: || Scenario::CompilesPerSession,
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("unchanged_reuse_reads_no_file"),
        word: Word::Succeeds,
        input: || Scenario::Unchanged,
        expect: Expect::Value((0, 0)),
    },
    Case {
        name: CaseName("a_body_edit_reads_only_that_file_and_keeps_the_cross_file_findings"),
        word: Word::Succeeds,
        input: || Scenario::BodyEdited,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName(
            "renaming_an_export_reads_that_file_alone_and_makes_the_cross_file_findings_again",
        ),
        word: Word::Succeeds,
        input: || Scenario::ExportRenamed,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_failed_run_keeps_the_previous_state"),
        word: Word::Succeeds,
        input: || Scenario::FailedRun,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_file_whose_bytes_could_not_be_read_is_read_again_by_the_next_run"),
        word: Word::Succeeds,
        input: || Scenario::UnreadableAgain,
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("a_changed_query_recompiles_once_and_then_reuses_the_plan"),
        word: Word::Succeeds,
        input: || Scenario::ChangedQueryKeepsOnePlan,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("a_changed_fragment_replaces_the_retained_compiled_plan"),
        word: Word::Succeeds,
        input: || Scenario::ChangedFragmentPlan,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("editing_a_nested_query_module_invalidates_plan_and_persisted_results"),
        word: Word::Succeeds,
        input: || Scenario::ChangedNestedModule,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("interleaved_runs_naming_different_rules_compile_each_rule_once"),
        word: Word::Succeeds,
        input: || Scenario::InterleavedRuns,
        expect: Expect::Value((2, 0)),
    },
    Case {
        name: CaseName("many_runs_in_a_row_leave_a_bounded_store"),
        word: Word::Succeeds,
        input: || Scenario::ManyRuns,
        expect: Expect::Value((1, 0)),
    },
    Case {
        name: CaseName("compiles_each_rule_once"),
        word: Word::Succeeds,
        input: || Scenario::CompilesPerRule,
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("rows_of_the_rules_own_columns_are_written_up_to_the_limit"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                query: Arc::new(QueryLanguageFake::selecting_twice()),
                answer: AnswerShape::Rows(RowTotal(1)),
                ..violations()
            })
        },
        expect: Expect::Value((1, 1)),
    },
    Case {
        name: CaseName("rows_of_the_cross_file_steps_are_answered_as_their_own_columns"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                query: Arc::new(QueryLanguageFake::counting_across_files()),
                answer: AnswerShape::Rows(RowTotal(20)),
                ..violations()
            })
        },
        expect: Expect::Value((2, 1)),
    },
    Case {
        name: CaseName("two_rules_answer_every_row_of_each"),
        word: Word::Succeeds,
        input: || {
            Scenario::Lint(|| Given {
                rules: vec!["one", "two"],
                answer: AnswerShape::Rows(RowTotal(20)),
                ..violations()
            })
        },
        expect: Expect::Value((4, 1)),
    },
    Case {
        name: CaseName("a_kind_no_linked_language_has_is_refused_before_a_file_is_read"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                query: Arc::new(QueryLanguageFake::comparing_a_kind()),
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::UnknownKind { .. }))),
    },
    Case {
        name: CaseName("an_unknown_semantic_kind_is_refused_before_a_file_is_read"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| {
                let mut query = QueryLanguageFake::comparing_a_kind();
                for step in &mut query.plan.steps {
                    if let Operation::Select { from, .. } = &mut step.operation {
                        *from = PlanInput::Relation(Relation::SemanticSyntax);
                    }
                }
                Given {
                    query: Arc::new(query),
                    ..violations()
                }
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::UnknownKind { .. }))),
    },
    Case {
        name: CaseName("a_language_that_writes_a_node_past_its_file_is_refused_naming_it"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                language: LanguageFake {
                    out_of_range: true,
                    ..LanguageFake::default()
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Provider { .. }))),
    },
    Case {
        name: CaseName("a_run_that_holds_more_than_its_budget_is_refused_naming_the_rules"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                memory: MemoryFake {
                    available: 1 << 30,
                    resident: 2 << 30,
                    fails: false,
                    asked: Budget::Machine,
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LintError::MemoryBudgetExceeded { .. }))
        }),
    },
    Case {
        name: CaseName("a_glob_that_is_not_one_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                include: vec!["["],
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Rule { .. }))),
    },
    Case {
        name: CaseName("a_rule_without_an_id_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                ids: vec![" "],
                ..violations()
            })
        },
        expect: Expect::Error(LintError::BlankRuleId),
    },
    Case {
        name: CaseName("two_rules_with_one_id_are_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                rules: vec!["one", "two"],
                ids: vec!["same", "same"],
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LintError::DuplicateRuleId { .. }))
        }),
    },
    Case {
        name: CaseName("a_parameter_the_rule_gives_nothing_for_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| {
                let mut query = QueryLanguageFake::selecting();
                query.plan.parameters = vec![PlanParameter {
                    name: ParameterName("wanted".into()),
                    kind: ParameterType::Text,
                }];
                Given {
                    query: Arc::new(query),
                    ..violations()
                }
            })
        },
        expect: Expect::Refusal(|error| {
            selects(matches!(error, LintError::MissingParameter { .. }))
        }),
    },
    Case {
        name: CaseName("rule_that_does_not_validate_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                rules: vec![REFUSED],
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Query { .. }))),
    },
    Case {
        name: CaseName("a_repository_larger_than_the_budget_is_refused"),
        word: Word::Refuses,
        input: || {
            Scenario::Lint(|| Given {
                memory: MemoryFake {
                    available: 4,
                    resident: 0,
                    fails: false,
                    asked: Budget::Machine,
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::TooLarge { .. }))),
    },
    Case {
        name: CaseName("fails_when_corpus_fails"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                corpus_fails: true,
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Corpus { .. }))),
    },
    Case {
        name: CaseName("fails_when_facts_fail"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                facts: FactsFake::failing(),
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Facts { .. }))),
    },
    Case {
        name: CaseName("fails_when_an_operator_refuses"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                files: vec![SOURCES[0]],
                query: Arc::new(QueryLanguageFake::comparing_a_missing_column()),
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Rule { .. }))),
    },
    Case {
        name: CaseName("fails_when_the_findings_cannot_be_written"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                ends: Ends::Failed,
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Output { .. }))),
    },
    Case {
        name: CaseName("fails_when_the_reader_stopped"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                ends: Ends::Closed,
                ..violations()
            })
        },
        expect: Expect::Error(LintError::OutputClosed),
    },
    Case {
        name: CaseName("fails_when_scheduler_fails"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                scheduler: SchedulerFake { fails: true },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Scheduler { .. }))),
    },
    Case {
        name: CaseName("fails_when_language_port_fails"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                language: LanguageFake {
                    catalog: FakeCatalog::Fails,
                    ..LanguageFake::default()
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Language { .. }))),
    },
    Case {
        name: CaseName("fails_when_memory_port_fails"),
        word: Word::Fails,
        input: || {
            Scenario::Lint(|| Given {
                memory: MemoryFake {
                    fails: true,
                    ..MemoryFake::default()
                },
                ..violations()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Memory { .. }))),
    },
    Case {
        name: CaseName("fails_when_store_port_cannot_keep"),
        word: Word::Fails,
        input: || {
            Scenario::BrokenStore(|| LintStoreFake {
                unwritable: true,
                ..LintStoreFake::default()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Store { .. }))),
    },
    Case {
        name: CaseName("fails_when_store_port_cannot_read"),
        word: Word::Fails,
        input: || {
            Scenario::BrokenStore(|| LintStoreFake {
                unreadable: true,
                ..LintStoreFake::default()
            })
        },
        expect: Expect::Refusal(|error| selects(matches!(error, LintError::Store { .. }))),
    },
];

/// Lints three files with a rule that counts the files each function name is in, edits `a.rs` to `edit`, and lints again over the
/// same store: answers how many files the second lint read, and 1 when the cross-file findings were taken from the store as they were
/// kept, 0 when they were made again.
fn crossed_after(edit: (&'static str, &'static str)) -> Result<Value, Failure> {
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
fn unchanged() -> Value {
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
fn failed_run() -> Value {
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
fn edited() -> Result<Value, Failure> {
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
fn deleted() -> Result<Value, Failure> {
    let store = Arc::new(LintStoreFake::default());
    let cold = again(&SOURCES, "root", "a rule", &store);
    let fewer = again(&SOURCES[..1], "root", "a rule", &store);
    assert_eq!(
        fewer.rows.len(),
        cold.rows.len() - 1,
        "and the one removed reports nothing"
    );
    assert!(fewer
        .rows
        .iter()
        .all(|row| row.0.iter().any(|cell| matches!(
            cell,
            Some(Typed::File(text) | Typed::Text(text)) if text.contains("a.rs")
        ))));
    read_and_unread(fewer)
}

/// Whether twenty runs in a row leave the store the size the fifth left it, as 1 or 0.
fn many_runs() -> Result<(usize, usize), Failure> {
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
fn placed() -> Result<Value, Failure> {
    let made = lint(violations());
    let first = made.rows.first().map(|row| &*row.0).unwrap_or_default();
    let (line, column) = match first {
        [Some(Typed::Text(_)), _, Some(Typed::Number(_)), .., line, column] => (line, column),
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
fn reading_the_project() -> QueryLanguageFake {
    let mut query = QueryLanguageFake::counting_across_files();
    query
        .plan
        .frontier
        .push(PlanInput::Relation(Relation::WorkspaceMember));
    query
}

/// A lint that counts what its steps are fed: how many times the language read the project, and how many rows the steps of the rules
/// were fed.
fn counted_lint(given: Given) -> Result<(usize, u64), Failure> {
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

/// Lints a rule that reads the project, over a manifest the run lists and a configuration above the root the language's lookup
/// reads, changes what `between` says, and lints again over the same store: answers how many times the second lint read the project,
/// and 1 when the steps of its rule were fed rows, 0 when it made nothing again.
fn consulted(between: Between) -> Result<Value, Failure> {
    let store = Arc::new(LintStoreFake::default());
    let manifest = ("project.manifest.fake", "members = 1");
    let configuration = ("../config.fake", "target = 1");
    let older = ("../older.fake", "target = 0");
    let given = |listed: (&'static str, &'static str),
                 outside: Vec<(&'static str, &'static str)>| Given {
        files: vec![SOURCES[0], SOURCES[1], listed],
        outside,
        store: Arc::clone(&store),
        root: "root",
        query: Arc::new(reading_the_project()),
        language: LanguageFake {
            // The lookup asks for the newer configuration first, then the older one.
            consulted: vec![FilePath(configuration.0.into()), FilePath(older.0.into())],
            ..LanguageFake::default()
        },
        ..violations()
    };
    let first = match between {
        Between::ConfigurationAppears => Vec::new(),
        Between::OlderBecomesNewer => vec![older],
        Between::Nothing
        | Between::ConfigurationAbove
        | Between::Manifest
        | Between::ConfigurationGone => vec![configuration],
    };
    let (read, fed) = counted_lint(given(manifest, first))?;
    assert_eq!(read, 1, "a cold run reads the project");
    assert!(fed > 0, "and feeds the steps of its rule");
    let (manifest, outside) = match between {
        Between::Nothing | Between::ConfigurationAppears => (manifest, vec![configuration]),
        Between::ConfigurationAbove => (manifest, vec![("../config.fake", "target = 2")]),
        Between::Manifest => (
            ("project.manifest.fake", "members = 2"),
            vec![configuration],
        ),
        Between::ConfigurationGone => (manifest, Vec::new()),
        Between::OlderBecomesNewer => (manifest, vec![configuration, older]),
    };
    let (read_again, fed_again) = counted_lint(given(manifest, outside))?;
    Ok((read_again, usize::from(fed_again > 0)))
}

#[test]
fn project_entry_observations_invalidate_persisted_absence_and_presence() {
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
}

/// A file that cannot be read stays unread, for the same reason, on the warm run.
fn unreadable_again() -> Result<Value, Failure> {
    let store = Arc::new(LintStoreFake::default());
    let files = [SOURCES[0], ("unstable.rs", "fn main() {}")];
    let cold = again(&files, "root", "a rule", &store);
    let warm = again(&files, "root", "a rule", &store);
    assert_eq!(
        warm.report, cold.report,
        "the file is unread for the same reason"
    );
    warm.report
        .map(|unread| (usize::from(warm.reads > 0), unread))
}

/// Two lints of one session: answers how many times the rules were compiled, and what the second left unread.
fn compiles_per_session() -> Result<Value, Failure> {
    let given = Given {
        files: vec![SOURCES[0]],
        rules: vec!["one", "two"],
        ..violations()
    };
    let (first, second) = (request(&given), request(&given));
    let world = world(given);
    let warm = world.use_case.lint(first)?;
    let report = world.use_case.lint(second)?;
    assert_eq!(
        warm.unread.len(),
        report.unread.len(),
        "a second lint leaves unread what the first did"
    );
    Ok((
        world.query.calls.load(Ordering::SeqCst),
        report.unread.len(),
    ))
}

/// The same root and outer module use a new nested-module body: both caches must observe it.
fn changed_nested_module() -> Result<Value, Failure> {
    let bundle = |revision: &str| {
        use ryo_values::domain::program::service::{document_source, query_bundle, source_id};
        let projection = if revision == "first" {
            "id"
        } else {
            "id as \"id\""
        };
        query_bundle(
            document_source(
                source_id("root").unwrap(),
                "import outer from \"outer\"; let found = outer(); emit found subject id key(id);",
            ),
            [
                document_source(
                    source_id("outer").unwrap(),
                    "import nested from \"nested\"; output relation<id: node>; let found = nested(); return found;",
                ),
                document_source(
                    source_id("nested").unwrap(),
                    format!("output relation<id: node>; let found = select {projection} from \"syntax-tree\"; return found;").into_boxed_str(),
                ),
            ],
        )
        .unwrap()
    };
    let given = violations();
    let mut first = request(&given);
    first.rules[0].query = bundle("first");
    let mut second = request(&given);
    second.rules[0].query = bundle("second");
    let world = world(given);
    let _first_report = world.use_case.lint(first)?;
    let after_first = world.language.parses.load(Ordering::SeqCst);
    let _second_report = world.use_case.lint(second)?;
    let parsed_again = world.language.parses.load(Ordering::SeqCst) > after_first;
    let compiled_twice = world.query.calls.load(Ordering::SeqCst) == 2;
    Ok((usize::from(parsed_again && compiled_twice), 0))
}

/// A module-only predicate edit changes actual findings through a retained real compiler plan.
fn changed_fragment_plan() -> Result<Value, Failure> {
    use crate::outbound::query_language::factory;
    use ryo_execution::inbound::use_cases::execution::types::query_language::QueryLanguagePort;
    use ryo_execution::inbound::use_cases::execution::types::CompiledPlans;

    let make_bundle = |prefix: &str| {
        let module = format!(
            "input files: relation<path: file>;\noutput relation<path: file>;\nlet selected = select path from files where starts_with(path, \"{prefix}\");\nreturn selected;"
        );
        ryo_values::domain::program::service::query_bundle(
            ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("root").unwrap(),
                "import choose from \"query/filter\"; let files = select path from \"filesystem.file\"; let result = choose(files=files); emit result subject \"path\" key(\"path\");",
            ),
            [ryo_values::domain::program::service::document_source(
                ryo_values::domain::program::service::source_id("query/filter").unwrap(),
                module.into_boxed_str(),
            )],
        )
        .unwrap()
    };
    let given = Given {
        files: vec![("alpha.txt", "alpha"), ("beta.txt", "beta")],
        ..violations()
    };
    let mut first = request(&given);
    first.rules[0].query = make_bundle("alpha");
    let mut second = request(&given);
    second.rules[0].query = make_bundle("beta");
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
    let query_language: Arc<dyn QueryLanguagePort> =
        Arc::new(factory::use_case(compile, typecheck));
    let scheduler: Arc<dyn SchedulerPort> = Arc::new(SchedulerFake::default());
    let mut world = scheduled_world_with_query(given, scheduler, WorkerCount(1), query_language);
    let plans = CompiledPlans::default();
    world.use_case = world.use_case.compiling_into(plans.clone());

    let _first_report = world.use_case.lint(first)?;
    let first_paths = collected_file_paths(&world);
    let _second_report = world.use_case.lint(second.clone())?;
    let second_paths = collected_file_paths(&world);
    let _retained_report = world.use_case.lint(second)?;
    let retained_paths = collected_file_paths(&world);
    Ok((
        usize::from(
            first_paths == std::collections::BTreeSet::from(["alpha.txt".to_owned()])
                && second_paths == std::collections::BTreeSet::from(["beta.txt".to_owned()])
                && retained_paths == second_paths,
        ),
        0,
    ))
}

/// Takes the file subjects written by one lint run and clears the shared collector.
fn collected_file_paths(world: &World) -> std::collections::BTreeSet<String> {
    let rows = std::mem::take(
        &mut world
            .collected
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .rows,
    );
    rows.iter()
        .flat_map(|row| row.0.iter())
        .filter_map(|cell| match cell {
            Some(ryo_values::domain::records::types::Cell::File(path)) => Some(path.to_string()),
            _ => None,
        })
        .collect()
}

/// Lints rule `a`, then rule `b`, then each again, over plans one session shares: answers how many times a rule was compiled.
fn interleaved_runs() -> Result<Value, Failure> {
    let rule = |id| Given {
        files: vec![SOURCES[0]],
        rules: vec!["one"],
        ids: vec![id],
        ..violations()
    };
    let world = world(rule("a"));
    for id in ["a", "b", "a", "b"] {
        let _run = world.use_case.lint(request(&rule(id)))?;
    }
    Ok((world.query.calls.load(Ordering::SeqCst), 0))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let fresh = || Arc::new(LintStoreFake::default());
    match scenario {
        Scenario::Lint(given) => {
            let made = lint(given());
            let findings = made.rows.len();
            made.report.map(|unread| (findings, unread))
        }
        Scenario::Placed => placed(),
        Scenario::Projects => {
            let made = lint(violations());
            made.report.map(|unread| (made.projects, unread))
        }
        Scenario::Warm => {
            serial_and_parallel_agree();
            let store = fresh();
            let cold = again(&SOURCES, "root", "a rule", &store);
            let warm = again(&SOURCES, "root", "a rule", &store);
            assert_eq!(cold.parses, 3, "a cold run reads every file");
            assert_eq!(warm.rows, cold.rows, "and answers the same findings");
            assert_eq!(&warm.report, &cold.report, "and the same unread files");
            read_and_unread(warm)
        }
        Scenario::Edited => edited(),
        Scenario::Unchanged => Ok(unchanged()),
        Scenario::BodyEdited => crossed_after(("a.rs", "fn a(){ }")),
        Scenario::ExportRenamed => crossed_after(("a.rs", "fn z() {}")),
        Scenario::FailedRun => Ok(failed_run()),
        Scenario::UnreadableAgain => unreadable_again(),
        Scenario::Consulted(between) => consulted(between),
        Scenario::Deleted => deleted(),
        Scenario::ChangedRule => {
            let store = fresh();
            let cold = again(&SOURCES, "root", "a rule", &store);
            assert_eq!(cold.parses, 3, "a cold run reads every file");
            read_and_unread(again(&SOURCES, "root", "another rule", &store))
        }
        Scenario::BrokenStore(store) => {
            read_and_unread(again(&SOURCES, "root", "a rule", &Arc::new(store())))
        }
        Scenario::CompilesPerSession => compiles_per_session(),
        Scenario::ChangedQueryKeepsOnePlan => {
            let rule = |query| Given {
                files: vec![SOURCES[0]],
                rules: vec![query],
                ..violations()
            };
            let (first, second) = (request(&rule("one")), request(&rule("two")));
            let world = world(rule("one"));
            let _first = world.use_case.lint(first)?;
            let _second = world.use_case.lint(second.clone())?;
            let _cached_second = world.use_case.lint(second)?;
            Ok((
                usize::from(world.query.calls.load(Ordering::SeqCst) == 2),
                0,
            ))
        }
        Scenario::ChangedFragmentPlan => changed_fragment_plan(),
        Scenario::ChangedNestedModule => changed_nested_module(),
        Scenario::InterleavedRuns => interleaved_runs(),
        Scenario::ManyRuns => many_runs(),
        Scenario::CompilesPerRule => {
            let made = lint(Given {
                rules: vec!["one", "two"],
                ..violations()
            });
            made.report.map(|unread| (made.compiles, unread))
        }
    }
}

#[test]
fn succeeds() {
    check(&{
        let analysis_corpus: Arc<dyn crate::outbound::corpus::interface::CorpusPort> =
            Arc::new(CorpusFake::default());
        let analysis_language: Arc<dyn crate::outbound::language::interface::LanguagePort> =
            Arc::new(LanguageFake::default());
        let analysis_memory: Arc<dyn crate::outbound::memory::interface::MemoryPort> =
            Arc::new(MemoryFake::default());
        let analysis_digest: Arc<
            dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort,
        > = Arc::new(DigestFake);
        LintUseCaseImpl::new(
            crate::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            crate::application::cross_file::service::CrossFile::new(Arc::clone(&analysis_corpus), std::sync::Arc::new(crate::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())))),
            Arc::clone(&analysis_corpus),
            Arc::new(FactsFake::default()),
            Arc::clone(&analysis_language),
            Arc::new(MatchingFake),
            Arc::new(QueryLanguageFake::selecting()),
            Arc::clone(&analysis_memory),
            Arc::new(crate::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            )),
            std::sync::Arc::new(crate::outbound::derived::factory::use_case(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            )),
            Arc::new(SchedulerFake::default()),
            WorkerCount(1),
            Arc::new(rows::factory::collect(rows::types::Collector::default())),
            Arc::clone(&analysis_digest),
            Arc::new(LintStoreFake::default()),
        )
    });
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn fails() {
    run(CASES, Word::Fails, subject);
}

#[test]
fn project_reuse_frames_root_and_repository_and_keeps_dependency_evidence() {
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
        let report = made.use_case.counting().lint(asked).expect("complete lint");
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
}

#[test]
fn consulted_equal_byte_files_revalidate_on_each_executable_metadata_change() {
    use ryo_values::domain::files::types::ExecutablePermission;
    for before in [false, true] {
        for changed in [false, true] {
            let store = Arc::new(LintStoreFake::default());
            let given = |executable| Given {
                files: vec![
                    SOURCES[0],
                    SOURCES[1],
                    ("project.manifest.fake", "members = 1"),
                ],
                outside: vec![("../config.fake", "target = 1")],
                outside_executable: if executable {
                    ExecutablePermission::PosixExecutable
                } else {
                    ExecutablePermission::PosixNotExecutable
                },
                store: Arc::clone(&store),
                root: "root",
                query: Arc::new(reading_the_project()),
                language: LanguageFake {
                    consulted: vec![FilePath("../config.fake".into())],
                    ..LanguageFake::default()
                },
                ..violations()
            };
            assert_eq!(counted_lint(given(before)).unwrap().0, 1);
            assert_eq!(
                counted_lint(given(if changed { !before } else { before }))
                    .unwrap()
                    .0,
                usize::from(changed),
                "consulted metadata must invalidate even when source digest is equal"
            );
        }
    }
}

/// What one whole lint answered: the rows written, what the rules could not decide, the files a rule skipped, and how many files the
/// language parsed.
struct Answered {
    rows: Vec<rows::types::Row>,
    undecided: String,
    skipped: String,
    parses: usize,
}

/// A lint of `files` with the cross-file rule, over `kept_in`, when the language rejects the syntax of `rejected`.
fn answered(
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
fn store_of(
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

const THREE: [(&str, &str); 3] = [
    ("a.rs", "fn a() {}"),
    ("b.rs", "fn b() {}"),
    ("c.rs", "fn c() {}"),
];

/// A cold, a warm and an edited run answer the rows and findings a cold run over the same files answers, whichever store keeps
/// them; the stores that keep read only the files that changed, the one that discards reads every file each time.
#[test]
fn cold_warm_and_edited_runs_answer_the_same_in_every_store() {
    use ryo_values::domain::runs::types::RuleApplicability::Always;
    let edited = [("a.rs", "fn changed() {}"), THREE[1], THREE[2]];
    for answer in [AnswerShape::Findings, AnswerShape::Rows(RowTotal(20))] {
        for (kind, parsed) in [
            ("memory", [3, 0, 1]),
            ("postcard", [3, 0, 1]),
            ("discard", [3, 3, 3]),
        ] {
            let directory = tempfile::tempdir().expect("a directory");
            let kept_in = store_of(kind, directory.path());
            for (run, files) in [&THREE[..], &THREE[..], &edited[..]]
                .into_iter()
                .enumerate()
            {
                let oracle = answered(
                    files,
                    &[],
                    Always,
                    answer,
                    store_of("discard", directory.path()),
                );
                let now = answered(files, &[], Always, answer, Arc::clone(&kept_in));
                assert_eq!(
                    now.rows, oracle.rows,
                    "{kind} {answer:?} run {run}: the rows"
                );
                assert_eq!(
                    now.undecided, oracle.undecided,
                    "{kind} run {run}: what was undecided"
                );
                assert_eq!(
                    now.skipped, oracle.skipped,
                    "{kind} run {run}: what was skipped"
                );
                // The reuse of a rows answer is not asserted: it is decided apart from the findings (see ryo-l1w3).
                if answer == AnswerShape::Findings {
                    assert_eq!(now.parses, parsed[run], "{kind} run {run}: the files read");
                }
            }
        }
    }
}

/// One file whose syntax a rule skips does not poison the cross-file partitions of the files around it: cold or warm, they answer
/// what the same run without that file answers, and the skip is named and kept.
#[test]
fn a_syntax_skipped_file_does_not_poison_unrelated_partitions_cold_or_warm() {
    use ryo_values::domain::runs::types::RuleApplicability::{AcceptedSyntax, Always};
    let without = [THREE[0], THREE[2]];
    for kind in ["memory", "postcard"] {
        let directory = tempfile::tempdir().expect("a directory");
        let oracle = answered(
            &without,
            &[],
            Always,
            AnswerShape::Findings,
            store_of("discard", directory.path()),
        );
        let kept_in = store_of(kind, directory.path());
        let cold = answered(
            &THREE,
            &["b.rs"],
            AcceptedSyntax,
            AnswerShape::Findings,
            Arc::clone(&kept_in),
        );
        let warm = answered(
            &THREE,
            &["b.rs"],
            AcceptedSyntax,
            AnswerShape::Findings,
            kept_in,
        );
        assert_eq!(cold.rows, oracle.rows, "{kind}: cold rows");
        assert_eq!(warm.rows, oracle.rows, "{kind}: warm rows");
        assert_eq!(cold.undecided, "[]", "{kind}: nothing undecided cold");
        assert_eq!(warm.undecided, "[]", "{kind}: nothing undecided warm");
        assert!(
            cold.skipped.contains("b.rs") && cold.skipped.contains("SyntaxRejected"),
            "{kind}: {}",
            cold.skipped
        );
        assert_eq!(warm.skipped, cold.skipped, "{kind}: the skip is kept");
        assert_eq!(warm.parses, 0, "{kind}: a warm run reads no file");
    }
}

/// A file whose syntax a rule skips is Skipped with the reason the language gave, for the file-phase rule as for the cross-file one:
/// the rows of the files around it are those of the same run without it, and nothing is undecided; it is never answered complete.
#[test]
fn a_syntax_rejected_file_is_skipped_with_its_reason_and_never_undecided_or_complete() {
    use ryo_values::domain::runs::types::RuleApplicability::{AcceptedSyntax, Always};
    let lint = |files: &[(&'static str, &'static str)], rejected: &[&str], applicability| {
        let mut given = Given {
            files: files.to_vec(),
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
        let rows = std::mem::take(
            &mut *world
                .collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
        .rows;
        (
            rows,
            format!("{:?}", report.unknowns),
            format!("{:?}", report.skipped),
        )
    };
    let oracle = lint(&[THREE[0], THREE[2]], &[], Always);
    let (rows, undecided, skipped) = lint(&THREE, &["b.rs"], AcceptedSyntax);
    assert_eq!(rows, oracle.0, "the rows of the files around it");
    assert_eq!(undecided, "[]", "a rejected file is not undecided");
    assert!(
        skipped.contains("b.rs") && skipped.contains("SyntaxRejected"),
        "named with its reason: {skipped}"
    );
    assert!(
        oracle.2 == "[]",
        "a file that is read is not skipped: {}",
        oracle.2
    );
}

mod project_budget;
