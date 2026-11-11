//! Binding identity and file generations through the actual compiled request API.
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::service::FileRun;
use crate::application::file_dag::program::facade_runtime::file_execution::service::open_file;
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_domain::domain::rule_intent::types::CaptureIntent;
use ryo_execution_domain::domain::rule_intent::types::DiagnosticDelivery;
use ryo_execution_domain::domain::rule_intent::types::EmitDemand;
use ryo_execution_domain::domain::rule_intent::types::FileRuleIntent;
use ryo_execution_domain::domain::rule_intent::types::ObservationDemand;
use ryo_execution_domain::domain::rule_intent::types::RuleDemand;
use ryo_execution_domain::domain::{
    coverage::types::RuleIndex,
    observation_schedule::types::RuleSchedule,
    operating::errors::OperatingError,
    prepared_request::types::{Compiled, ObservationIdentity},
    table_model::types::Rows,
};
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::{kinds, to_bits},
    types::{ColumnsView, FactColumn, FileId, Integer, Relation, Value as Cell},
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{
        ColumnIndex, Input as PlanInput, Operand, ParameterId, ParameterName, PlanComparison,
        RepeatedRows, StepId, StepScope, Test, WholeNumber,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
    query_bindings::types::{ParameterBinding, ParameterType, PlanParameter},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::Table,
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::sync::Arc;

/// The one parameter every generation plan binds.
const MAJOR: &str = "major";
/// The ids of the generation rules, in the order a request lists them.
const RULE_IDS: [&str; 2] = ["generation-first", "generation-second"];
/// A rule's empty query text and parameter spelling.
const NO_TEXT: &str = "";
/// The one file every generation request reads.
const GENERATION_FILE: &str = "generation";
/// Why the first root reads its provider.
const FIRST_ROOT_READS: &str = "the actual file generation consults its provider";
/// Why the second prepared root reads nothing.
const SECOND_ROOT_REPLAYS: &str = "the prepared second root replays its own bound result";
/// Why a fresh generation reads its provider.
pub(super) const FRESH_GENERATION_READS: &str = "a fresh file generation consults its provider";
/// Why the transferred output is independent.
pub(super) const TRANSFERRED_INDEPENDENT: &str = "transferred first output remains independent";
/// Why a compiled file rule must complete.
const MUST_COMPLETE: &str = "compiled file rule must complete";

/// A prepared program and the services its files run on.
pub(super) struct Prepared {
    /// The program every file generation opens.
    pub(super) prepared:
        Arc<crate::application::file_dag::program::preparation::types::PreparedProgram>,
    /// The services shared by the generations.
    pub(super) services: Arc<ProgramServices>,
}

/// How many times a rule consulted its provider.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Reads(pub(super) usize);

/// What a rule answered, and what it read to answer.
pub(super) struct RuleRun {
    /// The rule's emitted table.
    pub(super) table: Table,
    /// The provider reads it took.
    pub(super) reads: Reads,
}
fn binding_plan() -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![PlanParameter {
            name: ParameterName(MAJOR.into()),
            kind: ParameterType::Number,
        }],
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: PlanInput::Relation(Relation::LanguageTarget),
                    comparisons: vec![PlanComparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Parameter(ParameterId(0)),
                        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                    }],
                    kept: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(0)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: PlanInput::Step(StepId(1)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            },
        ],
        emit: Emit {
            step: StepId(2),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}
fn rule(id: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(id.into()),
                text: NO_TEXT.into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters(NO_TEXT.into()),
        include: vec![],
        exclude: vec![],
    }
}
pub(super) fn prepare(bindings: Vec<Vec<ParameterBinding>>) -> Prepared {
    let count = bindings.len();
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; count],
            plans: vec![binding_plan(); count],
            bindings,
        },
        &RULE_IDS[..count]
            .iter()
            .map(|id| rule(id))
            .collect::<Vec<_>>(),
        &vec![Digest([0; 16]); count],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath(GENERATION_FILE.into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    Prepared { prepared, services }
}
pub(super) fn language_columns(file: FileId, major: Integer) -> Vec<FactColumn> {
    [
        Cell::File(file),
        Cell::Number(major),
        Cell::Number(Integer(10)),
    ]
    .into_iter()
    .map(|cell| FactColumn(vec![to_bits(cell).0]))
    .collect()
}
pub(super) fn run_rule(
    file: &mut FileRun<'_>,
    rule: RuleIndex,
    storage: &[FactColumn],
    symbols: &mut Symbols,
) -> RuleRun {
    let reads = std::cell::Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        if relation == Relation::LanguageTarget {
            Ok(Rows::Stored(ColumnsView {
                kinds: kinds(relation),
                columns: storage,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let outcome = file
        .rule(
            rule,
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )
        .unwrap();
    let RootOutcome::Complete(table) = outcome.emit else {
        panic!("{MUST_COMPLETE}")
    };
    RuleRun {
        table,
        reads: Reads(reads.get()),
    }
}
fn identical_binding_dependent_operations_do_not_share_rule_graph_results() -> Result<Value, Failure>
{
    let Prepared { prepared, services } = prepare(vec![
        vec![ParameterBinding::Number(WholeNumber(3))],
        vec![ParameterBinding::Number(WholeNumber(2))],
    ]);
    let program = prepared.program();
    let first = program.rules()[0].original_steps[0]
        .as_ref()
        .unwrap()
        .logical;
    let second = program.rules()[1].original_steps[0]
        .as_ref()
        .unwrap()
        .logical;
    assert_ne!(
        program.logical_regions()[first.0],
        program.logical_regions()[second.0]
    );
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[
            FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            },
            FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            },
        ],
        services,
        ryo_values::domain::files::types::SourceView(b""),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let storage = language_columns(FileId(7), Integer(3));
    let RuleRun {
        table: first,
        reads: first_reads,
    } = run_rule(&mut file, RuleIndex(0), &storage, &mut symbols);
    assert!(first_reads.0 > 0, "{FIRST_ROOT_READS}");
    let prepared_work = file.readiness_counts();
    let RuleRun {
        table: second,
        reads: second_reads,
    } = run_rule(&mut file, RuleIndex(1), &storage, &mut symbols);
    assert_eq!(first.cells, vec![Cell::File(FileId(7))]);
    assert_eq!(second.count.0, 0);
    assert_eq!(second_reads, Reads(0), "{SECOND_ROOT_REPLAYS}");
    assert_eq!(file.readiness_counts(), prepared_work);
    file.finish().unwrap();
    Ok(1)
}

/// A scenario: what a row asks of the generation owner.
type Input = fn() -> Result<Value, Failure>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// This part has no failure of its own.
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("identical_binding_dependent_operations_do_not_share_rule_graph_results"),
    word: Word::Succeeds,
    input: || identical_binding_dependent_operations_do_not_share_rule_graph_results,
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod generations;
mod layout;
pub(crate) mod operators;
mod request_graph;
mod self_join_recogniser;
