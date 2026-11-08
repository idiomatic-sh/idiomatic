//! Every operation executes through the real prepare/open_file/collect-or-deliver protocol.
use ryo_execution_runtime::application::file_dag::program::file_session::interface::FileExecution as _;
use ryo_execution_runtime::application::file_dag::program::request_execution::interface::ExecutionRequest as _;
use crate::inbound::use_cases::execution::interface::ExecutionUseCase as _;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use super::super::super::fixtures::requests::Authored;
use super::super::super::fixtures::requests::Preparation;
use ryo_execution_runtime::outbound::facts::types::ResidentStability;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::inputs;
use super::super::super::fixtures as owner_fixtures;
use ryo_execution_runtime::application::file_dag::program::file_session::types::FileInput;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceLanguage;
use ryo_execution_runtime::application::file_dag::program::file_session::types::SourceRows;
use ryo_execution_runtime::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::inbound::use_cases::execution::interface::CaptureIntent;
use crate::inbound::use_cases::execution::interface::DiagnosticDelivery;
use crate::inbound::use_cases::execution::interface::EmitDemand;
use crate::inbound::use_cases::execution::interface::FileRuleIntent;
use crate::inbound::use_cases::execution::interface::ObservationDemand;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::RuleDemand;
use crate::inbound::use_cases::execution::interface::RuleIndex;
use crate::inbound::use_cases::execution::interface::SemanticLookup;
use ryo_execution_domain::domain::prepared_request::types::ObservationIdentity;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::*;
use ryo_execution_domain::domain::operating::fixtures::ExecutionObservation;
use ryo_execution_domain::domain::operator::service::prepare_operator;
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::CellKind;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plans::types::Operation;
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
    pub(crate) visits: Vec<u64>,
    pub(crate) scratch_cells: usize,
}
pub(crate) fn same_rows(table: &Table, rows: &[Vec<Cell>]) -> bool {
    table.count.0 == rows.len()
        && rows.iter().all(|row| row.len() == table.kinds.len())
        && table.cells == rows.iter().flatten().copied().collect::<Vec<_>>()
}
pub(crate) fn compare_delivery(
    operation: &Operation,
    inputs: &[Table],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    stop: Option<usize>,
) -> Result<ObservedDelivery, OperatingError> {
    compare_delivery_with_semantic(
        operation,
        inputs,
        bindings,
        source,
        symbols,
        stop,
        None,
        RefusalOutcome::Failed,
    )
}
pub(crate) fn compare_delivery_with_semantic(
    operation: &Operation,
    inputs: &[Table],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    stop: Option<usize>,
    semantic: Option<&dyn SemanticLookup>,
    refusal: RefusalOutcome,
) -> Result<ObservedDelivery, OperatingError> {
    compare_delivery_with_port(
        operation,
        inputs,
        bindings,
        source,
        symbols,
        stop,
        semantic,
        refusal,
        Arc::new(owner_fixtures::operators()),
    )
}
/// This helper only drives supported owner APIs. It cannot invoke a primitive,
/// construct a permit, choose an execution strategy or supply an alternate executor.
pub(crate) fn compare_delivery_with_port(
    operation: &Operation,
    inputs: &[Table],
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
    let schema = ryo_execution_domain::domain::operation_schema::service::schema_of(
        operation,
        &inputs
            .iter()
            .map(|table| ryo_execution_domain::domain::operation_schema::types::InputShape {
                kinds: &table.kinds,
                tree: None,
            })
            .collect::<Vec<_>>(),
        bindings,
    );
    let refs = inputs.iter().collect::<Vec<_>>();
    let authored = if matches!(operation, Operation::SpanBetween { .. }) {
        use ryo_values::domain::plan_parts::types::{Input, JoinColumn, Side, StepId};
        let left = Table {
            kinds: vec![CellKind::Span],
            cells: vec![inputs[0].cells[0]],
            count: ryo_values::domain::rows::types::RowCount(1),
        };
        let right = Table {
            kinds: vec![CellKind::Span],
            cells: vec![inputs[0].cells[1]],
            count: ryo_values::domain::rows::types::RowCount(1),
        };
        let join = Operation::Join {
            left: Input::Relation(
                ryo_relations::domain::relations::types::Relation::SourceLocation,
            ),
            right: Input::Relation(
                ryo_relations::domain::relations::types::Relation::StringLiteral,
            ),
            left_on: vec![],
            right_on: vec![],
            output: vec![
                JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                },
                JoinColumn {
                    side: Side::Right,
                    column: ColumnIndex(0),
                },
            ],
            filter: vec![],
            view: vec![],
            reach: None,
        };
        let mut a = inputs::table_inputs(join, &[&left, &right], bindings, symbols);
        let mut span = operation.clone();
        ryo_values::domain::plans::service::each_input(&mut span, &mut |input| {
            *input = Input::Step(a.plan.emit.step)
        });
        a.plan
            .steps
            .push(ryo_values::domain::plans::types::PlanStep {
                scope: ryo_values::domain::plan_parts::types::StepScope::PerFile,
                operation: span,
            });
        a.plan.emit.step = StepId((a.plan.steps.len() - 1) as u32);
        a
    } else {
        inputs::table_inputs(operation.clone(), &refs, bindings, symbols)
    };
    let emit = authored.plan.emit.step;
    let engine = Preparation::new(
        port,
        Arc::new(owner_fixtures::derived()),
    );
    let preparation = engine.prepare(Authored { plans: vec![authored.plan], bindings: vec![bindings.to_vec()], rules: vec![RuleSpec {
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
        }], digests: vec![Digest([0; 16])], hosted: authored.hosted, observations: ObservationIdentity::Independent });
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
                visits: vec![0; inputs.len()],
                scratch_cells: 0,
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
    let request = owner_fixtures::requests::execution().execution(&program, &corpus);
    let elements = Elements::default();
    let input = |diagnostic| FileInput {
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
    };
    let collected_intent = [FileRuleIntent::Run {
        diagnostic: DiagnosticDelivery::Collected,
        observations: ObservationDemand::NotRequired,
        capture: CaptureIntent::Disabled,
    }];
    let mut collected_file = request.open_file(input(&collected_intent), symbols)?;
    let outcome = collected_file.rule(
        RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        symbols,
    )?;
    let (collected, observation) = match outcome.emit {
        RootOutcome::Complete(table) => (
            Ok(table),
            collected_file.route_observation(RuleIndex(0), emit)?,
        ),
        RootOutcome::Failed(error) => {
            assert_eq!(
                refusal,
                RefusalOutcome::Failed,
                "actual Failed({error:?}) must match its authored classification"
            );
            (Err(error), ExecutionObservation::default())
        }
        RootOutcome::Unknown(error) => {
            assert_eq!(
                refusal,
                RefusalOutcome::Unknown,
                "actual Unknown({error:?}) must match its authored classification"
            );
            (Err(error), ExecutionObservation::default())
        }
        _ => panic!("declared per-file operation must settle"),
    };
    let visits =
        collected_file.counts(RuleIndex(0))?.fed[emit.0 as usize].0[..inputs.len()].to_vec();
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
        visits,
        scratch_cells: observation.row_capacity,
    })
}
fn all_registrations_refuse_a_foreign_variant() -> Result<(), Failure> {
    for (at, kind) in OperationKind::ALL.iter().enumerate() {
        let foreign = fixture(OperationKind::ALL[(at + 1) % OperationKind::ALL.len()]);
        assert!(matches!(
            prepare_operator(*kind, &foreign.operation, &[], &[]),
            Err(OperatingError::WrongOperation)
        ));
    }
    Ok(())
}
pub(crate) fn repeated(table: &Table, times: usize) -> Table {
    Table {
        kinds: table.kinds.clone(),
        cells: table.cells.repeat(times),
        count: ryo_values::domain::rows::types::RowCount(table.count.0 * times),
    }
}
type Input = fn() -> Result<Value, Failure>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(all_registrations_refuse_a_foreign_variant)),
        word: Word::Succeeds,
        input: || all_registrations_refuse_a_foreign_variant,
        expect: Expect::Value(()),
    },];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
