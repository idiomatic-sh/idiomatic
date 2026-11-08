//! What a row of the `lint` cases gives the use case: the ports, the rules and the files, and the world built over them.
use crate::outbound::language::fixtures::fake::LanguageFake;
use crate::outbound::lint_store::fixtures::fake::LintStoreFake;
use crate::outbound::memory::fixtures::fake::MemoryFake;
use crate::outbound::row_sink::fixtures::fake::Ends;
use crate::outbound::row_sink;
use crate::outbound::scheduler::fixtures::fake::SchedulerFake;
use ryo_execution_runtime::outbound::facts::fixtures::fake::FactsFake;
use ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::hosted::types::HostColumn;
use ryo_values::domain::hosted::types::HostColumnName;
use ryo_values::domain::hosted::types::HostKind;
use ryo_values::domain::hosted::types::HostName;
use ryo_values::domain::hosted::types::HostRelation;
use ryo_values::domain::hosted::types::HostRow;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::HostedNumber;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::LiteralText;
use ryo_values::domain::plan_parts::types::Operand;
use ryo_values::domain::plan_parts::types::PlanComparison;
use ryo_values::domain::plan_parts::types::QueryLiteral;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::Test;
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::row_texts::types::Cell as Typed;
use ryo_values::domain::runs::types::AnswerShape;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::Session;
use std::sync::Arc;
use super::super::errors::LintError;
use super::super::types::LintRequest;
use super::super::types::Rule;
use super::super::types::SourceProvider;
use super::super::use_case::LintUseCaseImpl;

/// What a row gives the use case: the ports it calls, the rules and the files linted.
pub(crate) struct Given {
    pub(crate) outside_executable: ryo_values::domain::files::types::ExecutablePermission,
    pub(crate) files: Vec<(&'static str, &'static str)>,
    /// Files on the disk the corpus does not list, such as those above the root.
    pub(crate) outside: Vec<(&'static str, &'static str)>,
    pub(crate) corpus_fails: bool,
    pub(crate) facts: FactsFake,
    pub(crate) language: LanguageFake,
    pub(crate) hosted: Vec<HostRelation>,
    pub(crate) memory: MemoryFake,
    pub(crate) query: Arc<QueryLanguageFake>,
    pub(crate) scheduler: SchedulerFake,
    pub(crate) ends: Ends,
    pub(crate) store: Arc<LintStoreFake>,
    /// A store of another kind the run keeps in instead of `store`, when a row is about the stores themselves.
    pub(crate) kept_in: Option<Arc<dyn crate::outbound::lint_store::interface::LintStorePort>>,
    pub(crate) root: &'static str,
    pub(crate) rules: Vec<&'static str>,
    pub(crate) ids: Vec<&'static str>,
    pub(crate) only: Vec<&'static str>,
    pub(crate) include: Vec<&'static str>,
    pub(crate) exclude: Vec<&'static str>,
    pub(crate) answer: AnswerShape,
    /// The provider the caller selects by name, when the row is about that selection.
    pub(crate) source_language: Option<&'static str>,
}

/// What a lint over a given made: the answer, the rows kept, the files the language read and what the ports were asked.
pub(crate) struct Lint {
    pub(crate) report: Result<usize, LintError>,
    pub(crate) rows: Vec<row_sink::types::KeptRow>,
    pub(crate) parses: usize,
    pub(crate) projects: usize,
    pub(crate) compiles: usize,
    /// How many files the corpus was asked to read.
    pub(crate) reads: usize,
}

pub(crate) const SOURCES: [(&str, &str); 3] = [
    ("a.rs", "fn a() {}"),
    ("b.rs", "fn b() {}"),
    ("notes.txt", "notes"),
];

/// A plan over `function`: every row, or only the rows whose name is the text given.
pub(crate) fn plan_with_name(name: &str) -> Plan {
    let comparison = PlanComparison {
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
pub(crate) fn plan_over_hosted() -> Plan {
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
pub(crate) fn host_placed() -> HostRelation {
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
pub(crate) fn violations() -> Given {
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
        source_language: None,
    }
}

/// The use case and the fakes whose counts a row reads.
pub(crate) struct World {
    pub(crate) use_case: LintUseCaseImpl,
    pub(crate) collected: row_sink::factory::CollectorConnection,
    pub(crate) language: Arc<LanguageFake>,
    pub(crate) query: Arc<QueryLanguageFake>,
    pub(crate) reads: Arc<std::sync::atomic::AtomicUsize>,
}

/// The request to lint what `given` stands for.
pub(crate) fn request(given: &Given) -> LintRequest {
    let globs = |texts: &[&str]| texts.iter().map(|glob| Glob((*glob).into())).collect();
    LintRequest {
        source_language: given.source_language.map_or(SourceProvider::Recognized, |name| {
            SourceProvider::Named(ryo_values::domain::descriptions::types::LanguageName(
                name.into(),
            ))
        }),
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
                query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(
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
