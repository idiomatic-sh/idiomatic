//! Span fixtures assemble canonical provider rows and run the actual compiled owner API.
use super::constants::{NO_TEXT, RULE_NAME};
use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::open_file;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::domain::rule_intent::types::CaptureIntent;
use crate::domain::rule_intent::types::DiagnosticDelivery;
use crate::domain::rule_intent::types::EmitDemand;
use crate::domain::rule_intent::types::FileRuleIntent;
use crate::domain::rule_intent::types::ObservationDemand;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::domain::rule_intent::types::RuleDemand;
use ryo_execution_store::application::file_dag::service::ScopePartials;
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
    files::types::{
        CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds, SourceView,
    },
    plan_parts::types::{ColumnIndex, Input, StepId, StepScope},
    plans::types::{Emit, Operation, Plan, PlanStep},
    query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::{RowCount, Table},
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::{collections::BTreeMap, sync::Arc};


/// One syntax row's file and span.
#[derive(Clone, Copy, Debug)]
pub(super) struct FileSpan {
    /// The file the row belongs to.
    pub file: FileId,
    /// The span the row covers.
    pub span: Span,
}

/// One relation and the rows a provider answers for it.
pub(super) struct Provider {
    /// The relation the provider answers.
    pub relation: Relation,
    /// The rows it answers with.
    pub table: Table,
}

/// Whether a span operation's output is also demanded as a stream by a following operation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Downstream {
    /// A downstream consumer demands the operation as a stream.
    Streamed,
    /// The operation's rows are collected.
    Collected,
}

/// How many matchers a run built.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Builds(pub usize);

/// What a run answered and how many matchers it built.
#[derive(Debug, PartialEq)]
pub(super) struct CompiledRun {
    /// The table the rule emitted, or why it did not.
    pub result: Result<Table, OperatingError>,
    /// The matchers it built.
    pub builds: Builds,
}

pub(super) fn rows(spans: &[FileSpan]) -> Table {
    Table {
        kinds: kinds(Relation::SyntaxTree).to_vec(),
        count: RowCount(spans.len()),
        cells: spans
            .iter()
            .enumerate()
            .flat_map(|(at, FileSpan { file, span })| {
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
pub(super) fn select(relation: Relation, columns: &[ColumnIndex]) -> PlanStep {
    step(Operation::Select {
        from: Input::Relation(relation),
        comparisons: vec![],
        kept: columns.to_vec(),
    })
}
/// The downstream Extend is an authored no-op row consumer; it demands the preceding
/// span operation as a stream through the same normal scheduler, rather than a test feed.
pub(super) fn run(
    mut steps: Vec<PlanStep>,
    providers: &[Provider],
    source: SourceView<'_>,
    symbols: &mut Symbols,
    downstream: Downstream,
) -> CompiledRun {
    if downstream == Downstream::Streamed {
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
        id: RuleId(RULE_NAME.into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(RULE_NAME.into()),
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
    };
    let derived = ryo_execution_store::application::file_dag::fixtures::program_derived_port();
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
        Err(error) => {
            return CompiledRun {
                result: Err(error),
                builds: Builds(0),
            };
        }
    };
    let result = (|| {
        let services = Arc::new(ProgramServices::new(
            Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
                crate::application::file_dag::fixtures::operators(),
            ))),
            derived,
            &[CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: FilePath(RULE_NAME.into()),
                size: FileSize(source.0.len() as u64),
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
            source,
            ryo_execution_store::application::file_dag::relations::types::ReuseEligibility::Eligible,
            symbols,
            None,
            None)?;
        let read = |relation| {
            providers
                .iter()
                .find(|provider| provider.relation == relation)
                .map(|provider| Rows::Computed(&provider.table))
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
    CompiledRun {
        result,
        builds: Builds(
            prepared
                .matchers()
                .builds
                .load(std::sync::atomic::Ordering::Relaxed),
        ),
    }
}
