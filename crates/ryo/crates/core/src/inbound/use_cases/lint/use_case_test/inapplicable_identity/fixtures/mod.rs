//! Two rules of one request, one producing a named observation and one consuming it, of which one skips the file the language rejects.
use super::super::*;
use crate::inbound::use_cases::lint::fixtures::given::World;
use crate::inbound::use_cases::lint::fixtures::given::request;
use crate::inbound::use_cases::lint::fixtures::given::violations;
use crate::inbound::use_cases::lint::fixtures::stores::THREE;
use crate::inbound::use_cases::lint::fixtures::stores::store_of;
use crate::inbound::use_cases::lint::fixtures::worlds::scheduled_world_with_query_resources;
use crate::inbound::use_cases::lint::interface::LintUseCase as _;
use ryo_execution::outbound::query_language::errors::QueryLanguageError;
use ryo_execution::outbound::query_language::interface::QueryLanguagePort;
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepScope;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, StepId};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::runs::types::{AnswerShape, RuleId};
use ryo_values::domain::{
    descriptions::types::ValidateReport,
    hosted::types::HostedRelation,
    observations::types::{
        BoundObservationInput, ObservationBinding, ObservationBindingName, ObservationColumn,
        ObservationColumnName, ObservationName, ObservationProducer, PlanObservation,
    },
    query_tokens::types::QueryBundle,
    row_texts::types::RowTotal,
    runs::types::RuleApplicability,
};
use std::sync::atomic::Ordering;

/// Which rule skips the file the language rejects.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Skipping {
    /// The rule that consumes the named observation: its inapplicable files differ from the producer's.
    Consumer,
    /// The rule that produces it.
    Producer,
}

/// The store the warm run is kept in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Kept {
    Memory,
    Postcard,
}

/// What the run after the cold one asks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Then {
    /// The findings again.
    Findings,
    /// The rows of the findings, after a findings run.
    Rows,
}

/// What a row asks: who skips, where the runs are kept, what the second run asks.
#[derive(Clone, Copy, Debug)]
pub(super) struct Scenario {
    pub skipping: Skipping,
    pub kept: Kept,
    pub then: Then,
}

/// Whether the second run answered from what the first kept or evaluated again.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Warm {
    /// No file was read and no step was fed a row.
    Reused,
    /// The steps of the rules were fed rows again.
    Evaluated,
}

struct Queries;

fn count_columns() -> Vec<ObservationColumn> {
    vec![
        ObservationColumn {
            name: ObservationColumnName("file".into()),
            kind: CellKind::File,
        },
        ObservationColumn {
            name: ObservationColumnName("count".into()),
            kind: CellKind::Number,
        },
    ]
}

impl QueryLanguagePort for Queries {
    fn validate(
        &self,
        bundle: &QueryBundle,
        hosted: &[HostedRelation],
        observations: &[BoundObservationInput],
    ) -> Result<ValidateReport, QueryLanguageError> {
        QueryLanguageFake::selecting().validate(bundle, hosted, observations)
    }
    fn compile(
        &self,
        bundle: &QueryBundle,
        _hosted: &[HostedRelation],
        observations: &[BoundObservationInput],
    ) -> Result<Plan, QueryLanguageError> {
        let first = &*bundle.root.text == "first";
        let mut plan = QueryLanguageFake::selecting().plan;
        plan.observation_inputs = observations.to_vec();
        let file = ryo_relations::domain::relations::service::kinds(Relation::Function)
            .iter()
            .position(|kind| *kind == CellKind::File)
            .expect("canonical Function file column");
        let file = ColumnIndex(u32::try_from(file).unwrap());
        plan.steps = if first {
            vec![PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::Function),
                    comparisons: vec![],
                    kept: vec![file],
                },
            }]
        } else {
            Vec::new()
        };
        let from = || {
            if first {
                PlanInput::Step(StepId(0))
            } else {
                PlanInput::BoundObservation(
                    ryo_values::domain::plan_parts::types::ObservationBindingId(0),
                )
            }
        };
        plan.frontier = vec![from()];
        plan.steps.push(PlanStep {
            scope: StepScope::CrossFile,
            operation: Operation::GroupCount {
                from: from(),
                keys: vec![ColumnIndex(0)],
                distinct: None,
            },
        });
        plan.emit = Emit {
            step: StepId(u32::try_from(plan.steps.len() - 1).unwrap()),
            subject: ColumnIndex(0),
            key: vec![ColumnIndex(1)],
            headings: vec![],
        };
        if first {
            plan.observations = vec![PlanObservation {
                name: ObservationName("counts".into()),
                source: PlanInput::Step(plan.emit.step),
                columns: count_columns(),
                scope: StepScope::CrossFile,
            }];
        }
        Ok(plan)
    }
}

/// What one lint answered.
#[derive(Debug, PartialEq)]
pub(super) struct Observed {
    pub rows: Vec<row_sink::types::KeptRow>,
    pub undecided: String,
    pub skipped: String,
    pub parses: usize,
    pub fed: u64,
}

fn lint_once(
    skipping: Skipping,
    answer: AnswerShape,
    kept_in: Arc<dyn crate::outbound::lint_store::interface::LintStorePort>,
) -> Observed {
    let mut given = Given {
        files: THREE.to_vec(),
        rules: vec!["first", "second"],
        ids: vec!["first", "second"],
        answer,
        kept_in: Some(kept_in),
        ..violations()
    };
    given.language.rejected_syntax = vec![FilePath("b.rs".into())];
    let mut asked = request(&given);
    asked.rules[1].observations = vec![ObservationBinding {
        name: ObservationBindingName("counts".into()),
        columns: count_columns(),
        producers: vec![ObservationProducer::Output {
            rule: RuleId("first".into()),
            output: ObservationName("counts".into()),
        }],
    }];
    let skipper = match skipping {
        Skipping::Consumer => 1,
        Skipping::Producer => 0,
    };
    asked.rules[skipper].applicability = RuleApplicability::AcceptedSyntax;
    let memory = Arc::new(MemoryFake::default());
    let operators = Arc::new(
        ryo_execution_runtime::outbound::operators::factory::use_case(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(
                std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case()),
            ),
        ),
    );
    let made = scheduled_world_with_query_resources(
        given,
        Arc::new(crate::outbound::scheduler::factory::sequential()),
        WorkerCount(1),
        Arc::new(Queries),
        memory,
        operators,
    );
    let World {
        use_case,
        collected,
        language,
        ..
    } = made;
    let report = use_case
        .counting(std::sync::Arc::new(
            ryo_execution_runtime::outbound::observer::factory::counting(),
        ))
        .lint(asked)
        .expect("the lint answers");
    Observed {
        rows: std::mem::take(
            &mut *collected
                .0
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
        )
        .rows,
        undecided: format!("{:?}", report.unknowns),
        skipped: format!("{:?}", report.skipped),
        parses: language.parses.load(Ordering::SeqCst),
        fed: report
            .work
            .iter()
            .flat_map(|rule| &rule.steps)
            .map(|step| step.rows.0)
            .sum(),
    }
}

/// A cold run, then another of the same files over the store the cold run kept: each answers what the same request answers over a
/// store that keeps nothing. Answers whether the second run reused or evaluated again.
pub(super) fn observe(scenario: Scenario) -> Warm {
    let directory = tempfile::tempdir().expect("a directory");
    let kind = match scenario.kept {
        Kept::Memory => "memory",
        Kept::Postcard => "postcard",
    };
    let kept_in = store_of(kind, directory.path());
    let rows = AnswerShape::Rows(RowTotal(20));
    let then = match scenario.then {
        Then::Findings => AnswerShape::Findings,
        Then::Rows => rows,
    };
    let discard = || store_of("discard", directory.path());
    let cold_oracle = lint_once(scenario.skipping, AnswerShape::Findings, discard());
    let warm_oracle = lint_once(scenario.skipping, then, discard());
    let cold = lint_once(
        scenario.skipping,
        AnswerShape::Findings,
        Arc::clone(&kept_in),
    );
    let warm = lint_once(scenario.skipping, then, kept_in);
    assert_eq!(cold.rows, cold_oracle.rows, "{scenario:?}: cold rows");
    assert_eq!(warm.rows, warm_oracle.rows, "{scenario:?}: warm rows");
    for (made, oracle, run) in [(&cold, &cold_oracle, "cold"), (&warm, &warm_oracle, "warm")] {
        assert_eq!(
            made.undecided, oracle.undecided,
            "{scenario:?}: {run} undecided"
        );
        assert_eq!(made.skipped, oracle.skipped, "{scenario:?}: {run} skipped");
    }
    // A producer that skips the rejected file leaves both rules undecided on it, cold and warm alike: that is the Phase 1 poisoning
    // (a syntax-rejected sibling makes every rule unknown on its file) and it is the same for a store that keeps nothing.
    if scenario.skipping == Skipping::Consumer {
        assert_eq!(warm.undecided, "[]", "{scenario:?}: nothing undecided warm");
    }
    if warm.parses == 0 && warm.fed == 0 {
        Warm::Reused
    } else {
        Warm::Evaluated
    }
}
