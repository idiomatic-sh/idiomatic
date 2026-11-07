//! The steps of the `lint` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;

use crate::Behavior;
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::{LintRequest, Rule};
use ryo_core::inbound::use_cases::lint::use_case::LintUseCaseImpl;
use ryo_core::outbound::corpus::fixtures::fake::CorpusFake;
use ryo_core::outbound::digest::fixtures::fake::DigestFake;
use ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake;
use ryo_language::domain::files::types::FilePath;
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::files::types::{CorpusFile, FileBytes};
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::Session;

use ryo_core::outbound::language::fixtures::fake::LanguageFake;
use ryo_core::outbound::matching::fixtures::fake::MatchingFake;
use ryo_core::outbound::memory::fixtures::fake::MemoryFake;
use ryo_core::outbound::query_language::fixtures::fake::QueryLanguageFake;
use ryo_core::outbound::row_sink;
use ryo_core::outbound::row_sink::interface::RowSinkPort;
use ryo_core::outbound::scheduler::fixtures::fake::SchedulerFake;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};

/// A file of the corpus at `path`, holding one function.
fn file(path: &str) -> CorpusFile {
    CorpusFile {
        executable: Default::default(),
        path: FilePath(path.into()),
        bytes: FileBytes(b"fn main() {}".to_vec()),
    }
}

/// A plan that keeps every row of the `function` relation.
fn every_function() -> Plan {
    Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: vec![PlanStep {
            operation: Operation::Select {
                from: Input::Relation(Relation::Function),
                comparisons: Vec::new(),
                kept: Vec::new(),
            },
            scope: StepScope::PerFile,
        }],
        emit: Emit {
            step: StepId(0),
            subject: ColumnIndex(2),
            key: vec![ColumnIndex(3)],
            headings: Vec::new(),
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    }
}

/// Names the scene the next `lint` step runs.
#[given(expr = "the lint input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `lint` over the scene the input names, keeping what it answered.
#[when(expr = "lint runs")]
fn runs(world: &mut Behavior) {
    let paths: Vec<&str> = match world.input.as_str() {
        "two_rust_files_and_a_text_file" => vec!["a.rs", "b.rs", "notes.txt"],
        "only_a_text_file" => vec!["notes.txt"],
        "broken_file" => vec!["a.rs", "broken.rs"],
        _ => vec![],
    };
    let corpus = CorpusFake {
        files: paths.iter().map(|path| file(path)).collect(),
        fails: world.input == "corpus_down",
        reads: Arc::default(),
        outside: Vec::new(),
    };
    let query_language = QueryLanguageFake {
        plan: every_function(),
        ..QueryLanguageFake::default()
    };
    let collected = row_sink::types::Collector::default();
    let kept: Arc<dyn RowSinkPort> = Arc::new(row_sink::factory::collect(collected.clone()));
    let use_case = {
        let analysis_corpus: Arc<dyn ryo_core::outbound::corpus::interface::CorpusPort> =
            Arc::new(corpus);
        let analysis_language: Arc<dyn ryo_core::outbound::language::interface::LanguagePort> =
            Arc::new(LanguageFake::default());
        let analysis_memory: Arc<dyn ryo_core::outbound::memory::interface::MemoryPort> =
            Arc::new(MemoryFake::default());
        let analysis_digest: Arc<
            dyn ryo_execution::inbound::use_cases::execution::types::digest::DigestPort,
        > = Arc::new(DigestFake);
        LintUseCaseImpl::new(
            ryo_core::application::file_analysis::service::FileAnalysis::new(
                Arc::clone(&analysis_corpus),
                Arc::clone(&analysis_language),
                Arc::clone(&analysis_digest),
                Arc::clone(&analysis_memory),
            ),
            ryo_core::application::cross_file::service::CrossFile::new(Arc::clone(
                &analysis_corpus,
            ), std::sync::Arc::new(ryo_core::outbound::publication::factory::execution(std::sync::Arc::new(ryo_execution::inbound::use_cases::publication::use_case::PublicationUseCaseImpl::new())))),
            Arc::clone(&analysis_corpus),
            Arc::new(ryo_core::outbound::facts::factory::memory()),
            Arc::clone(&analysis_language),
            Arc::new(MatchingFake),
            Arc::new(query_language),
            Arc::clone(&analysis_memory),
            Arc::new(ryo_core::outbound::operators::factory::use_case(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
            )),
            std::sync::Arc::new(ryo_core::outbound::derived::factory::use_case(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            )),
            Arc::new(SchedulerFake::default()),
            ryo_core::outbound::scheduler::types::WorkerCount(1),
            Arc::clone(&kept),
            Arc::clone(&analysis_digest),
            Arc::new(LintStoreFake::default()),
        )
    };
    let query = if world.input == "refused_text" {
        "refuse"
    } else {
        "a query"
    };
    let request = LintRequest {
        source_language: ryo_core::inbound::use_cases::lint::types::SourceProvider::Recognized,
        answer: ryo_values::domain::runs::types::AnswerShape::Findings,
        hosted: Vec::new(),
        session: Session {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(".".into()),
        },
        files: Vec::new(),
        rules: vec![Rule {
            observations: Vec::new(),
            id: RuleId("rule".into()),
            query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(
                query.into(),
            )),
            applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
            parameters: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        }],
    };
    world.answer = match use_case.lint(request) {
        Ok(report) => format!(
            "{} findings and {} unread",
            std::mem::take(
                &mut *collected
                    .0
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
            )
            .rows
            .len(),
            report.unread.len()
        ),
        Err(error) => error.to_string(),
    };
}

/// Asserts what the last `lint` step answered.
#[then(expr = "lint answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a step's `{string}` into an owned `String` and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "lint answered otherwise");
}
