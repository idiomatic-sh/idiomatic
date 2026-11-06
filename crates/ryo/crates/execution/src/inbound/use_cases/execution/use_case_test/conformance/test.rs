//! Every operation executes through the real prepare/open_file/collect-or-deliver protocol.
use super::super::super::{
    fixtures as owner_fixtures, interface::*, types::*, use_case::ExecutionUseCaseImpl,
};
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::*;
use crate::domain::operating::types::ExecutionObservation;
use crate::domain::operator::service::registered;
use ryo_language::domain::{
    elements::types::Elements,
    semantics::types::{RenderOutcome, RenderRequest},
    symbols::types::Symbols,
};
use ryo_relations::domain::relations::types::{CellKind, FileId, Value};
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    plans::{
        service::kind_of,
        types::{Operation, OperationKind},
    },
    rows::types::Table,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in super::super) enum RefusalOutcome {
    Failed,
    Unknown,
}
pub(in super::super) struct ObservedDelivery {
    pub(in super::super) schema: Result<Vec<CellKind>, OperatingError>,
    pub(in super::super) collected: Result<Table, OperatingError>,
    pub(in super::super) rows: Vec<Vec<Value>>,
    pub(in super::super) stream_error: Option<OperatingError>,
    pub(in super::super) visits: Vec<u64>,
    pub(in super::super) scratch_cells: usize,
}
pub(in super::super) fn same_rows(table: &Table, rows: &[Vec<Value>]) -> bool {
    table.count.0 == rows.len()
        && rows.iter().all(|row| row.len() == table.kinds.len())
        && table.cells == rows.iter().flatten().copied().collect::<Vec<_>>()
}
pub(in super::super) fn compare_delivery(
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
pub(in super::super) fn compare_delivery_with_semantic(
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
        Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ),
    )
}
/// This helper only drives supported owner APIs. It cannot invoke a primitive,
/// construct a permit, choose an execution strategy or supply an alternate executor.
pub(in super::super) fn compare_delivery_with_port(
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
        program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
        rule_requests::types::{RuleParameters, RuleSpec},
        runs::types::{RuleApplicability, RuleId},
        stored_facts::types::Digest,
    };
    let schema = crate::domain::operation_schema::service::schema_of(
        operation,
        &inputs
            .iter()
            .map(|table| crate::domain::operation_schema::types::InputShape {
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
        let mut a = owner_fixtures::table_inputs(join, &[&left, &right], bindings, symbols);
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
        owner_fixtures::table_inputs(operation.clone(), &refs, bindings, symbols)
    };
    let emit = authored.plan.emit.step;
    let engine = ExecutionUseCaseImpl::new(
        port,
        Arc::new(ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new()),
    );
    let preparation = engine.prepare(PreparedRequest {
        plans: vec![authored.plan],
        bindings: vec![bindings.to_vec()],
        hosted: authored.hosted,
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
    let request = engine.begin(&program, &corpus);
    let elements = Elements::default();
    let input = |diagnostic| FileInput {
        language: SourceLanguage::Recognized,
        resident_stability: facts::ResidentStability::Stable,
        file: FileId(0),
        evaluation: &[],
        publication: &[],
        intents: diagnostic,
        source,
        elements: &elements,
        read: &read,
        semantic,
        kinds: None,
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
            collected_file.inner.route_observation(RuleIndex(0), emit)?,
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
#[derive(Debug, Default)]
struct ContinuesAfterDriverRefusal(std::sync::atomic::AtomicUsize);
impl ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase
    for ContinuesAfterDriverRefusal
{
    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::interface::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::interface::OperatorAnswer,
        ryo_operators::inbound::use_cases::operators::interface::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::interface::{
            OperatorAnswer, OperatorCall, OperatorsUseCase, SetInput, SetKind,
        };
        match call {
            OperatorCall::SetRow {
                input:
                    SetInput::Values {
                        kind: SetKind::Union,
                        row,
                        driver,
                        ..
                    },
            } => {
                let first = driver.emit(row);
                // Union is granted distinct-state access, not a RHS membership index.
                let refusal = driver.contains_right(row);
                let later_emit = driver.emit(row);
                let later_lookup = driver.first_time(row);
                if first.is_ok() && refusal.is_err() && later_emit.is_err() && later_lookup.is_err()
                {
                    self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(OperatorAnswer::Written)
            }
            other => {
                ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new()
                    .operators(other)
            }
        }
    }
}
#[test]
fn engine_driver_preserves_first_refusal_and_denies_later_semantic_callbacks() {
    let mut case = fixture(OperationKind::Union);
    let port = Arc::new(ContinuesAfterDriverRefusal::default());
    let observed = compare_delivery_with_port(
        &case.operation,
        &case.inputs,
        &case.bindings,
        SOURCE,
        &mut case.symbols,
        None,
        None,
        RefusalOutcome::Failed,
        port.clone(),
    )
    .expect("real Union prepares");
    assert_eq!(observed.collected.err(), Some(OperatingError::WrongBinding));
    assert_eq!(observed.stream_error, Some(OperatingError::WrongBinding));
    assert!(
        observed.rows.is_empty(),
        "a semantic prefix never becomes a publication"
    );
    assert_eq!(
        port.0.load(std::sync::atomic::Ordering::Relaxed),
        2,
        "both independently declared sessions exercise the actual engine driver"
    );
}
#[test]
fn every_registered_class_runs_real_collect_and_stream_with_independent_expectations() {
    let semantic = KnownSyntax;
    assert_eq!(OperationKind::ALL.len(), 26);
    for kind in OperationKind::ALL {
        let mut case = fixture(*kind);
        assert_eq!(kind_of(&case.operation), *kind);
        let observed = compare_delivery_with_semantic(
            &case.operation,
            &case.inputs,
            &case.bindings,
            SOURCE,
            &mut case.symbols,
            None,
            if *kind == OperationKind::RenderSyntax {
                Some(&semantic)
            } else {
                None
            },
            RefusalOutcome::Failed,
        )
        .unwrap();
        let collected = observed.collected.unwrap();
        assert_eq!(observed.stream_error, None, "{kind:?}");
        assert_eq!(observed.schema.unwrap(), collected.kinds);
        assert!(
            same_rows(&collected, &observed.rows),
            "ordered bag {kind:?}"
        );
        assert_expected(&case.expected, &observed.rows, &case.symbols);
    }
}

#[test]
fn unsupported_semantic_batch_publishes_no_prefix() {
    #[derive(Debug)]
    struct PartialSyntax;
    impl crate::domain::operating::types::SemanticLookup for PartialSyntax {
        fn render_batch(
            &self,
            _: &[RenderRequest],
            _: &[u8],
        ) -> Result<Vec<RenderOutcome>, OperatingError> {
            Ok(vec![
                RenderOutcome::Known(Arc::from("a")),
                RenderOutcome::Unsupported(ryo_relations::domain::terms::types::UnknownReason(24)),
            ])
        }
    }
    let mut case = fixture(OperationKind::RenderSyntax);
    ryo_values::domain::rows::service::push_row(
        &mut case.inputs[0],
        &[Value::Node(
            ryo_relations::domain::relations::service::node_id(
                ryo_relations::domain::relations::types::FileId(0),
                ryo_relations::domain::relations::types::Position(1),
            ),
        )],
    );
    let observed = compare_delivery_with_semantic(
        &case.operation,
        &case.inputs,
        &case.bindings,
        SOURCE,
        &mut case.symbols,
        None,
        Some(&PartialSyntax),
        RefusalOutcome::Unknown,
    )
    .unwrap();
    assert!(matches!(
        observed.collected,
        Err(OperatingError::SemanticIncomplete { .. })
    ));
    assert!(matches!(
        observed.stream_error,
        Some(OperatingError::SemanticIncomplete { .. })
    ));
    assert!(observed.rows.is_empty());
}

#[test]
fn all_registrations_refuse_a_foreign_variant() {
    for (at, kind) in OperationKind::ALL.iter().enumerate() {
        let foreign = fixture(OperationKind::ALL[(at + 1) % OperationKind::ALL.len()]);
        assert!(matches!(
            registered(*kind).prepare(&foreign.operation, &[], &[]),
            Err(OperatingError::WrongOperation)
        ));
    }
}
pub(in super::super) fn repeated(table: &Table, times: usize) -> Table {
    Table {
        kinds: table.kinds.clone(),
        cells: table.cells.repeat(times),
        count: ryo_values::domain::rows::types::RowCount(table.count.0 * times),
    }
}
