//! The delivery protocol every conformance group drives: an authored plan through the real prepare, open_file, collect
//! and forward steps, and what it observed.
use super::super::super::fixtures as owner_fixtures;
use super::super::super::fixtures::requests::Authored;
use super::super::super::fixtures::requests::Preparation;
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase as _;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use crate::inbound::use_cases::execution::interface::SemanticLookup;
use crate::inbound::use_cases::prepare::interface::ObservationIdentity;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution as _;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RefusalOutcome {
    Failed,
    Unknown,
}
pub(crate) struct ObservedDelivery {
    pub(crate) schema: Result<Vec<CellKind>, OperatingError>,
    pub(crate) collected: Result<Table, OperatingError>,
    pub(crate) rows: Vec<Vec<Cell>>,
    pub(crate) stream_error: Option<OperatingError>,
}
/// Delivers an authored plan through the real prepare, open_file, collect and forward protocol.
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
    stop: Option<usize>,
    semantic: Option<&dyn SemanticLookup>,
    refusal: RefusalOutcome,
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
            assert_eq!(
                refusal,
                RefusalOutcome::Failed,
                "a metadata refusal cannot satisfy an expected runtime Unknown outcome"
            );
            return Ok(ObservedDelivery {
                schema,
                collected: Err(error.clone()),
                rows: vec![],
                stream_error: Some(error),
            });
        }
    };
    let corpus = [CorpusInput::Available(CorpusEntry {
        path: ryo_language::domain::files::types::FilePath("operator-contract.py".into()),
        size: FileSize(source.len() as u64),
        modified: Nanoseconds(0),
        executable: ExecutablePermission::Unknown,
    })];
    let read = |relation| {
        authored
            .facts
            .iter()
            .find(|(actual, _)| *actual == relation)
            .map(|(_, table)| SourceRows::Computed(table))
            .ok_or(OperatingError::NotRead)
    };
    let request = owner_fixtures::requests::execution().execution(&program, &corpus, None);
    let elements = Elements::default();
    let input = |diagnostic| {
        FileInput {
        language: SourceLanguage::Recognized,
        resident_stability: ResidentStability::Stable,
        file: FileId(0),
        evaluation: &[],
        publication: &[],
        intents: diagnostic,
        source: ryo_values::domain::files::types::SourceView(source),
        elements: &elements,
        read: &read,
        semantic: match semantic {
            Some(lookup) => ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Supplied(lookup),
            None => ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
        },
        kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
    }
    };
    let collected_intent = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }];
    let mut collected_file = request.open_file(input(&collected_intent), symbols)?;
    let outcome = collected_file.rule(
        RuleIndex(0),
        crate::inbound::use_cases::execution::fixtures::requests::emitted(),
        symbols,
    )?;
    let collected = match outcome.emit {
        RootOutcome::Complete(table) => Ok(table),
        RootOutcome::Failed(error) => {
            assert_eq!(
                refusal,
                RefusalOutcome::Failed,
                "actual Failed({error:?}) must match its authored classification"
            );
            Err(error)
        }
        RootOutcome::Unknown(error) => {
            assert_eq!(
                refusal,
                RefusalOutcome::Unknown,
                "actual Unknown({error:?}) must match its authored classification"
            );
            Err(error)
        }
        _ => panic!("declared per-file operation must settle"),
    };
    collected_file.finish()?;
    let forward_intent = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Forwarded,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }];
    let mut callback_file = request.open_file(input(&forward_intent), symbols)?;
    let mut rows = vec![];
    let stream_error = callback_file
        .emit_rows(RuleIndex(0), symbols, &mut |row| {
            if stop == Some(rows.len()) {
                return Err(OperatingError::WrongOperation);
            }
            rows.push(row.to_vec());
            Ok(())
        })
        .err();
    callback_file.finish()?;
    request.finish();
    Ok(ObservedDelivery {
        schema,
        collected,
        rows,
        stream_error,
    })
}
