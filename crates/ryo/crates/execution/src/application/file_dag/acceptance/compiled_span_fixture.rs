//! Span fixtures assemble canonical provider rows and run the actual compiled owner API.
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::open_file;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::application::file_dag::program::protocol::types::CaptureIntent;
use crate::application::file_dag::program::protocol::types::DiagnosticDelivery;
use crate::application::file_dag::program::protocol::types::EmitDemand;
use crate::application::file_dag::program::protocol::types::FileRuleIntent;
use crate::application::file_dag::program::protocol::types::ObservationDemand;
use crate::application::file_dag::program::protocol::types::RootOutcome;
use crate::application::file_dag::program::protocol::types::RuleDemand;
use crate::application::file_dag::scope::ScopePartials;
use crate::domain::{
    coverage::types::RuleIndex,
    file_program::types::RuleSchedule,
    operating::errors::OperatingError,
    prepared_request::types::{Compiled, ObservationIdentity},
    table_model::types::Rows,
};
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::kinds,
    types::{FileId, NodeId, Relation, Span, Value},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Emit, Operation, Plan, PlanStep},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::{RowCount, Table},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::{collections::BTreeMap, sync::Arc};

pub(super) fn rows(spans: &[(FileId, Span)]) -> Table {
    Table {
        kinds: kinds(Relation::SyntaxTree).to_vec(),
        count: RowCount(spans.len()),
        cells: spans
            .iter()
            .enumerate()
            .flat_map(|(at, (file, span))| {
                [
                    Value::Node(NodeId(at as u64)),
                    Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                    Value::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                    Value::File(*file),
                    Value::Span(*span),
                ]
            })
            .collect(),
    }
}
pub(super) fn step(operation: Operation) -> PlanStep {
    PlanStep {
        operation,
        scope: StepScope::PerFile,
    }
}
pub(super) fn select(relation: Relation, columns: &[u32]) -> PlanStep {
    step(Operation::Select {
        from: Input::Relation(relation),
        comparisons: vec![],
        kept: columns.iter().copied().map(ColumnIndex).collect(),
    })
}
/// The downstream Extend is an authored no-op row consumer; it demands the preceding
/// span operation as a stream through the same normal scheduler, rather than a test feed.
pub(super) fn run(
    mut steps: Vec<PlanStep>,
    providers: &[(Relation, Table)],
    source: &[u8],
    symbols: &mut Symbols,
    downstream: bool,
) -> (Result<Table, OperatingError>, usize) {
    if downstream {
        steps.push(step(Operation::Extend {
            from: Input::Step(StepId((steps.len() - 1) as u32)),
            values: vec![],
        }));
    }
    let plan = Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        emit: Emit {
            step: StepId((steps.len() - 1) as u32),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        steps,
        frontier: vec![],
        comparison_origins: vec![],
    };
    let rule = RuleSpec {
        id: RuleId("span-contract".into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId("span-contract".into()),
                text: "".into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    };
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = match crate::application::file_dag::program::preparation::service::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan],
            bindings: vec![vec![]],
        },
        &[rule],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    ) {
        Ok(prepared) => prepared,
        Err(error) => return (Err(error), 0),
    };
    let result = (|| {
        let services = Arc::new(ProgramServices::new(
            Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case())),
            ))),
            derived,
            &[CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: FilePath("span-contract.py".into()),
                size: FileSize(source.len() as u64),
                modified: Nanoseconds(0),
            })],
        ));
        let mut file = open_file(
            &prepared,
            FileId(0),
            ScopePartials::File {
                evaluation: &[],
                publication: &[],
            },
            &[FileRuleIntent::Run {
                diagnostic: DiagnosticDelivery::Collected,
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }],
            services,
            ryo_values::domain::files::types::SourceView(source),
            crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
            symbols,
            None,
            None)?;
        let read = |relation| {
            providers
                .iter()
                .find(|(provider, _)| *provider == relation)
                .map(|(_, table)| Rows::Computed(table))
                .ok_or(OperatingError::NotRead)
        };
        let outcome = file.rule(
            RuleIndex(0),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )?;
        let result = match outcome.emit {
            RootOutcome::Complete(table) => Ok(table),
            RootOutcome::Unknown(error) | RootOutcome::Failed(error) => Err(error),
            _ => Err(OperatingError::NotRun),
        };
        file.finish()?;
        result
    })();
    (
        result,
        prepared
            .matchers()
            .builds
            .load(std::sync::atomic::Ordering::Relaxed),
    )
}
