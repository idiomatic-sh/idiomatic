//! The delivery protocol every conformance group drives: an authored plan through the real prepare, then file 0 run through
//! `run_file` with the rule's diagnostic collected, and what it observed.
use super::super::super::fixtures as owner_fixtures;
use super::super::super::fixtures::requests::Authored;
use super::super::super::fixtures::requests::Preparation;
use super::super::super::fixtures::requests::RunFile;
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase as _;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::SemanticLookup;
use crate::inbound::use_cases::execution::interface::{EmittedRoot, FileEnd, FileRules, RuleAfter};
use crate::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::inputs;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ProgramRequestExecutionPort as _;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

/// How the collected root settled.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Settlement {
    /// The root completed with a table.
    Complete,
    /// Preparation refused the plan, or the root failed.
    Failed,
    /// The root's answer is unknown.
    Unknown,
    /// The root did not settle (partial, skipped, no query or deferred); `collected` is then `NotRun`.
    Unsettled,
}
pub(crate) struct ObservedDelivery {
    pub(crate) settlement: Settlement,
    pub(crate) schema: Result<Vec<CellKind>, OperatingError>,
    pub(crate) collected: Result<Table, OperatingError>,
    pub(crate) rows: Vec<Vec<Cell>>,
    pub(crate) stream_error: Option<OperatingError>,
}
/// Delivers an authored plan through the real prepare and the one per-file route, collected.
#[allow(
    clippy::too_many_arguments,
    reason = "the delivery protocol's inputs, each read once, as compare_delivery_with_port passes them"
)]
pub(crate) fn deliver(
    authored: inputs::AuthoredRows,
    schema: Result<Vec<CellKind>, OperatingError>,
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    semantic: Option<&dyn SemanticLookup>,
    port: Arc<dyn OperatorsPort>,
) -> Result<ObservedDelivery, OperatingError> {
    use ryo_values::domain::{
        files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
        query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
        stored_facts::types::Digest,
    };
    let engine = Preparation::new(port, Arc::new(owner_fixtures::derived()));
    let preparation = engine.prepare(Authored {
        plans: vec![authored.plan],
        bindings: vec![bindings.to_vec()],
        rules: vec![RuleSpec {
            id: RuleId("operation-conformance".into()),
            query: QueryBundle {
                root: QueryDocumentSource {
                    id: QuerySourceId("operation-conformance".into()),
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
        }],
        digests: vec![Digest([0; 16])],
        hosted: authored.hosted,
        observations: ObservationIdentity::Independent,
    });
    let program = match preparation {
        Ok(program) => program,
        Err(error) => {
            return Ok(ObservedDelivery {
                settlement: Settlement::Failed,
                schema,
                collected: Err(error.clone()),
                rows: vec![],
                stream_error: Some(error),
            });
        }
    };
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
        size: FileSize(u64::try_from(source.len()).expect("a source length fits in u64")),
        modified: Nanoseconds(0),
        executable: ExecutablePermission::Unknown,
    })];
    let mut facts = owner_fixtures::TableFacts::new(
        &authored
            .facts
            .iter()
            .map(|(relation, table)| (*relation, table))
            .collect::<Vec<_>>(),
    );
    let request = owner_fixtures::requests::execution().execution(&program, &corpus, None.into());
    let elements = Elements::default();
    let mut root = |diagnostic, symbols: &mut Symbols| -> Result<RootOutcome, OperatingError> {
        let FileRules { rules, end, .. } = owner_fixtures::requests::run_file(
            &request,
            RunFile {
                source,
                elements: &elements,
                intents: &[FileRuleIntent::Run {
                    diagnostic,
                    observations: ObservationDemand::NotRequired,
                    capture: CaptureIntent::DiagnosticAndObservations,
                }],
                semantic,
            },
            symbols,
            &mut facts,
        );
        if let FileEnd::Refused(stop) = end {
            return Err(owner_fixtures::facts::operating(stop));
        }
        let ran = rules.into_iter().next().ok_or(OperatingError::NotRun)?;
        match (ran.emit, ran.after) {
            (EmittedRoot::Delivered(outcome), _) => Ok(outcome),
            (EmittedRoot::Withheld, RuleAfter::Stopped(stop)) => {
                Err(owner_fixtures::facts::operating(stop))
            }
            (EmittedRoot::Withheld, RuleAfter::Ran { .. } | RuleAfter::Unreached) => {
                Err(OperatingError::NotRun)
            }
        }
    };
    let (settlement, collected) = match root(DiagnosticDelivery::Collected, symbols)? {
        RootOutcome::Complete(table) => (Settlement::Complete, Ok(table)),
        RootOutcome::Failed(error) => (Settlement::Failed, Err(error)),
        RootOutcome::Unknown(error) => (Settlement::Unknown, Err(error)),
        RootOutcome::Partial { .. }
        | RootOutcome::Skipped(_)
        | RootOutcome::NoQuery
        | RootOutcome::Deferred { .. } => (Settlement::Unsettled, Err(OperatingError::NotRun)),
    };
    // `run_file` delivers a root collected, the one diagnostic delivery on the production route (a forwarded root has none: the
    // file run refuses it), so the delivered rows are the collected table's and the delivery's refusal is the collected one.
    let (rows, stream_error) = match &collected {
        Ok(table) => (
            table
                .cells
                .chunks(table.kinds.len().max(1))
                .take(table.count.0)
                .map(<[Cell]>::to_vec)
                .collect(),
            None,
        ),
        Err(error) => (vec![], Some(error.clone())),
    };
    request.finish();
    Ok(ObservedDelivery {
        settlement,
        schema,
        collected,
        rows,
        stream_error,
    })
}
