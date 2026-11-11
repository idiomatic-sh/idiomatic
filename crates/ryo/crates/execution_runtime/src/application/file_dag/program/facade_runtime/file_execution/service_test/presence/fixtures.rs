//! The support of the `presence` cases.
pub(super) mod routing;
use crate::application::file_dag::execution::service::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::{
    service::FileRun, service::open_file,
};
use crate::application::file_dag::program::facade_runtime::service::ProgramServices;
use crate::application::file_dag::program::facade_runtime::types::RegionProbe;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::application::file_dag::service_test::operators::fixtures::NO_TEXT;
use ryo_execution_domain::domain::coverage::types::{Partial, RuleIndex};
use ryo_execution_domain::domain::executable_program::types::RegionId;
use ryo_execution_domain::domain::observation_schedule::types::RuleSchedule;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::prepared_request::types::{Compiled, ObservationIdentity};
use ryo_execution_domain::domain::rule_intent::types::{
    CaptureIntent, DiagnosticDelivery, EmitDemand, FileRuleIntent, ObservationDemand, RuleDemand,
};
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::service::ScopePartials;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::service::kinds;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Flag, NodeId, Relation, Span, Value,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds, SourceView,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, ComparisonOrigin, Input, JoinColumn, LiteralText, Operand, PlanComparison,
    QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
};
use ryo_values::domain::plans::types::{Emit, Operation, Plan, PlanStep};
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rows::types::{RowCount, Table};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use ryo_values::domain::stored_facts::types::Digest;
use std::sync::Arc;

/// The fixture file's source text: no name reads it, every text is derived.
pub(super) const SOURCE: &[u8] = b"";

/// The text column of both fixture relations: `comment.text` and `function.name`.
pub(super) const TEXT: u32 = 1;

pub(super) fn equal(column: u32, text: &str) -> PlanComparison {
    compare(column, Test::Equal, text)
}

pub(super) fn compare(column: u32, test: Test, text: &str) -> PlanComparison {
    PlanComparison {
        test,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
        origin: ComparisonOrigin::Unrecorded,
    }
}

pub(super) fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}

pub(super) fn select(from: Input, comparisons: Vec<PlanComparison>) -> PlanStep {
    step(Operation::Select {
        from,
        comparisons,
        kept: vec![],
    })
}

pub(super) fn at(step: u32) -> Input {
    Input::Step(StepId(step))
}

/// A join of two steps on the file column, keeping the left columns.
pub(super) fn join_on_file(left: u32, right: u32, width: u32) -> PlanStep {
    step(Operation::Join {
        left: at(left),
        right: at(right),
        left_on: vec![ColumnIndex(2)],
        right_on: vec![ColumnIndex(2)],
        output: (0..width)
            .map(|column| JoinColumn {
                side: Side::Left,
                column: ColumnIndex(column),
            })
            .collect(),
        filter: vec![],
        view: vec![],
        reach: None,
    })
}

pub(super) fn plan(steps: Vec<PlanStep>) -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: StepId(u32::try_from(steps.len() - 1).unwrap_or(0)),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
    }
}

/// Rule B: comments whose text is "y", in a file with a function. It cannot match a file whose comments say "x".
pub(super) fn rule_b() -> Plan {
    plan(vec![
        select(Input::Relation(Relation::Comment), vec![equal(TEXT, "y")]),
        select(Input::Relation(Relation::Function), vec![]),
        join_on_file(0, 1, 5),
    ])
}

pub(super) fn spec(at: usize) -> RuleSpec {
    let id = format!("presence-{at}");
    RuleSpec {
        id: RuleId(id.clone().into()),
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

pub(super) fn prepare(plans: Vec<Plan>) -> Arc<PreparedProgram> {
    let count = plans.len();
    let specs: Vec<RuleSpec> = (0..count).map(spec).collect();
    crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; count],
            plans,
            bindings: vec![vec![]; count],
        },
        &specs,
        &vec![Digest([0; 16]); count],
        &[],
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
        ObservationIdentity::Independent,
    )
    .expect("the fixture program prepares")
}

pub(super) fn services() -> Arc<ProgramServices> {
    Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(Arc::new(
            crate::application::file_dag::fixtures::operators(),
        ))),
        ryo_execution_store::application::file_dag::fixtures::program_derived_port(),
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("presence".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ))
}

/// One fixture file: its comments' and functions' texts, interned as derived names.
pub(super) struct Facts {
    pub(super) symbols: Symbols,
    pub(super) comment: Table,
    pub(super) function: Table,
}

pub(super) fn span(at: usize) -> Value {
    Value::Span(Span {
        start: ryo_relations::domain::relations::types::Offset(u32::try_from(at).unwrap_or(0)),
        end: ryo_relations::domain::relations::types::Offset(u32::try_from(at).unwrap_or(0)),
    })
}

pub(super) fn facts(comments: &[&str], functions: &[&str]) -> Facts {
    let mut symbols = Symbols::default();
    let mut name = |text: &str| {
        Value::Symbol(ryo_language::domain::symbols::service::intern_derived(
            &mut symbols,
            ByteSlice(SOURCE),
            NameText(text),
        ))
    };
    let comment = table(
        Relation::Comment,
        comments
            .iter()
            .enumerate()
            .map(|(at, text)| {
                vec![
                    Value::Node(NodeId(u64::try_from(at).expect("a row number fits in u64"))),
                    name(text),
                    Value::File(FileId(0)),
                    span(at),
                    Value::Flag(Flag::Lowered),
                ]
            })
            .collect(),
    );
    let function = table(
        Relation::Function,
        functions
            .iter()
            .enumerate()
            .map(|(at, text)| {
                vec![
                    Value::Node(NodeId(
                        100 + u64::try_from(at).expect("a row number fits in u64"),
                    )),
                    name(text),
                    Value::File(FileId(0)),
                    span(at),
                ]
            })
            .collect(),
    );
    Facts {
        symbols,
        comment,
        function,
    }
}

pub(super) fn table(relation: Relation, rows: Vec<Vec<Value>>) -> Table {
    Table {
        kinds: kinds(relation).to_vec(),
        count: RowCount(rows.len()),
        cells: rows.into_iter().flatten().collect(),
    }
}

pub(super) fn run_intent() -> FileRuleIntent {
    FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }
}

/// What one file run answered and left behind.
pub(super) struct Ran {
    pub(super) outcomes: Vec<Result<RootOutcome, OperatingError>>,
    pub(super) fed: Vec<Vec<u64>>,
    pub(super) probes: Vec<RegionProbe>,
    pub(super) presence_absent: u64,
    pub(super) again: Vec<Result<(), OperatingError>>,
    pub(super) cleanup: Result<(), OperatingError>,
    pub(super) dormant: usize,
    pub(super) finished: Result<(), OperatingError>,
}

/// Runs every rule of `prepared` on one file in rule order, then probes `regions` and finishes the file.
pub(super) fn run(
    prepared: &Arc<PreparedProgram>,
    facts: &mut Facts,
    intents: &[FileRuleIntent],
    partial: &[Partial],
    regions: &[RegionId],
) -> Ran {
    let Facts {
        symbols,
        comment,
        function,
    } = facts;
    let location = table(Relation::SourceLocation, vec![]);
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the file supplies three relations of the catalog and refuses every other as unread"
    )]
    let read = |relation| match relation {
        Relation::Comment => Ok(Rows::Computed(&*comment)),
        Relation::Function => Ok(Rows::Computed(&*function)),
        Relation::SourceLocation => Ok(Rows::Computed(&location)),
        _ => Err(OperatingError::NotRead),
    };
    let mut file: FileRun<'_> = open_file(
        prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: partial,
            publication: &[],
        },
        intents,
        services(),
        SourceView(SOURCE),
        ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
        symbols,
        None,
        None,
    )
    .expect("the file opens");
    let mut outcomes = Vec::new();
    let mut fed = Vec::new();
    for (rule, intent) in intents.iter().enumerate() {
        if !matches!(intent, FileRuleIntent::Run { .. }) {
            outcomes.push(Err(OperatingError::NotRun));
            fed.push(Vec::new());
            continue;
        }
        let outcome = file.rule(
            RuleIndex(rule),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        );
        fed.push(
            file.collected_counts(RuleIndex(rule))
                .map(|counts| counts.fed.iter().map(|each| each.0.iter().sum()).collect())
                .unwrap_or_default(),
        );
        outcomes.push(outcome.map(|outcome| outcome.emit));
    }
    let probes = regions
        .iter()
        .map(|region| file.region_probe(*region))
        .collect();
    let presence_absent = file.readiness_counts().presence_absent.0;
    let cleanup = file.cleanup_status();
    let again = regions
        .iter()
        .map(|region| file.retire_again(*region))
        .collect();
    let dormant = file.state_counts().dormant_obligations.0;
    let finished = file.finish().map(|_| ());
    Ran {
        outcomes,
        fed,
        probes,
        presence_absent,
        again,
        cleanup,
        dormant,
        finished,
    }
}

/// The region each step of a rule runs in.
pub(super) fn regions_of(prepared: &PreparedProgram, rule: usize) -> Vec<RegionId> {
    let program = prepared.program();
    program.rules()[rule]
        .original_steps
        .iter()
        .map(|origin| {
            let origin = origin.as_ref().expect("every step is placed");
            program.logical_regions()[origin.logical.0]
        })
        .collect()
}
