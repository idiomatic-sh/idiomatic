//! Delivery through the owner when its operator port keeps answering after a refusal.
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use super::super::super::fixtures as owner_fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use crate::inbound::use_cases::execution::interface::SemanticLookup;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::*;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::values::KnownSyntax;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::expectations::SOURCE;
use ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::expectations::assert_expected;
use ryo_language::domain::semantics::types::RenderOutcome;
use ryo_language::domain::semantics::types::RenderRequest;
use ryo_relations::domain::relations::types::FileId;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::domain::plans::service::kind_of;
use std::sync::Arc;
use super::super::conformance::test::compare_delivery_with_semantic;
use super::super::conformance::test::compare_delivery_with_port;
use super::super::conformance::test::same_rows;
use super::super::conformance::test::RefusalOutcome;
use super::super::conformance::test::ObservedDelivery;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::rows::types::Table;

/// Delivers an operation whose listed inputs have no rows while the region that computes it runs. Each such input is
/// the difference of its authored rows with themselves: its presence is that of the authored rows, which have some, so
/// the operation's kernel is evaluated over an empty input instead of being found absent.
fn compare_emptied(
    operation: &Operation,
    inputs: &[Table],
    emptied: &[usize],
    bindings: &[ryo_values::domain::query_bindings::types::ParameterBinding],
    source: &[u8],
    symbols: &mut Symbols,
    semantic: Option<&dyn SemanticLookup>,
) -> Result<ObservedDelivery, OperatingError> {
    use ryo_values::domain::plan_parts::types::{Input, StepId, StepScope};
    use ryo_values::domain::plans::types::PlanStep;
    let schema = super::super::conformance::test::schema(operation, inputs, bindings);
    let mut authored = super::super::conformance::test::author(operation, inputs, bindings, symbols);
    let mut last = authored
        .plan
        .steps
        .pop()
        .expect("the operation under proof is the authored plan's last step");
    let mut sources = Vec::new();
    ryo_values::domain::plans::service::each_input(&mut last.operation, &mut |input| sources.push(*input));
    let mut replaced = Vec::new();
    for (at, source) in sources.iter().enumerate() {
        if emptied.contains(&at) {
            replaced.push(Input::Step(StepId(authored.plan.steps.len() as u32)));
            authored.plan.steps.push(PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Difference { left: *source, right: *source },
            });
        } else {
            replaced.push(*source);
        }
    }
    let mut at = 0;
    ryo_values::domain::plans::service::each_input(&mut last.operation, &mut |input| {
        *input = replaced[at];
        at += 1;
    });
    authored.plan.steps.push(last);
    authored.plan.emit.step = StepId((authored.plan.steps.len() - 1) as u32);
    super::super::conformance::test::deliver(
        authored,
        schema,
        inputs.len(),
        bindings,
        source,
        symbols,
        None,
        semantic,
        RefusalOutcome::Failed,
        Arc::new(owner_fixtures::operators()),
    )
}

/// Each operation's presence declaration rests on its kernel: a required input with no rows, the others as authored, gives
/// no row; a union of two empty inputs gives none; and a reach join over no edge finds no pair, so closure is not
/// reflexive. Each emptied input is computed empty inside a region that runs, so the kernel answers, not the presence
/// decision.
fn every_required_input_empty_answers_no_rows() -> Result<(), Failure> {
    use ryo_execution_domain::domain::operator::contract::presence::PresenceMask;
    let semantic = KnownSyntax;
    let rows = |case: &mut ryo_execution_runtime::application::file_dag::acceptance::operator_contract_test::fixtures::expectations::Fixture, emptied: &[usize]| {
        compare_emptied(
            &case.operation,
            &case.inputs,
            emptied,
            &case.bindings,
            SOURCE,
            &mut case.symbols,
            Some(&semantic),
        )
        .map(|observed| (observed.collected.map(|table| table.count.0), observed.rows.len()))
    };
    for kind in OperationKind::ALL {
        let presence = ryo_execution_domain::domain::operator::contract::service::semantics(*kind).presence;
        let width = fixture(*kind).inputs.len();
        let emptied: Vec<Vec<usize>> = match presence {
            PresenceMask::AllOf(mask) => (0..width)
                .filter(|at| mask.0 & (1 << at) != 0)
                .map(|at| vec![at])
                .collect(),
            PresenceMask::AnyOf(mask) => {
                vec![(0..width).filter(|at| mask.0 & (1 << at) != 0).collect()]
            }
            PresenceMask::Always => continue,
        };
        assert!(!emptied.is_empty(), "{kind:?} declares a required input its fixture lacks");
        for inputs in emptied {
            let mut case = fixture(*kind);
            assert_eq!(rows(&mut case, &inputs)?, (Ok(0), 0), "{kind:?} with inputs {inputs:?} empty");
        }
    }
    let mut reach = fixture(OperationKind::Join);
    if let ryo_values::domain::plans::types::Operation::Join { reach: lookup, .. } = &mut reach.operation {
        *lookup = Some(ryo_values::domain::plan_parts::types::ReachLookup {
            source: ryo_values::domain::plan_parts::types::ColumnIndex(0),
            destination: ryo_values::domain::plan_parts::types::ColumnIndex(1),
            keys: vec![],
            columns: vec![
                ryo_values::domain::plan_parts::types::ColumnIndex(0),
                ryo_values::domain::plan_parts::types::ColumnIndex(1),
            ],
        });
    }
    assert_eq!(rows(&mut reach, &[1])?, (Ok(0), 0), "a reach join over no edge");
    Ok(())
}
type Input = fn() -> Result<Value, Failure>;
type Value = ();
type Failure = OperatingError;
#[derive(Debug, Default)]
struct ContinuesAfterDriverRefusal(std::sync::atomic::AtomicUsize);
impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort
    for ContinuesAfterDriverRefusal
{
    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
        use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
        use ryo_operators::domain::set_rows::errors::SetRowsError;
        use ryo_operators::domain::evaluation::types::EvaluationInput;
        match call {
            OperatorCall::Evaluate {
                operation: ryo_values::domain::plans::types::Operation::Union { .. },
                input: EvaluationInput::Row(row),
                driver,
                ..
            } => {
                let driver = driver.set_rows();
                let first = driver.emit(row);
                // Union is granted distinct-state access, not a RHS membership index.
                let refusal = driver.contains_right(row);
                let later_emit = driver.emit(row);
                let later_lookup = driver.first_time(row);
                if matches!(first, Ok(()))
                    && matches!(refusal, Err(SetRowsError::Interrupted))
                    && matches!(later_emit, Err(SetRowsError::Interrupted))
                    && matches!(later_lookup, Err(SetRowsError::Interrupted))
                {
                    self.0.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
                }
                Ok(OperatorAnswer::Written)
            }
            other => {
                owner_fixtures::operators().operators(other)
            }
        }
    }
}
fn engine_driver_preserves_first_refusal_and_denies_later_semantic_callbacks() -> Result<(), Failure> {
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
    Ok(())
}
fn every_registered_class_runs_real_collect_and_stream_with_independent_expectations() -> Result<(), Failure> {
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
    Ok(())
}
fn unsupported_semantic_batch_publishes_no_prefix() -> Result<(), Failure> {
    #[derive(Debug)]
    struct PartialSyntax;
    impl ryo_execution_domain::domain::operating::types::SemanticLookup for PartialSyntax {
        fn render_batch(
            &self,
            _: &[RenderRequest],
            _: ryo_values::domain::files::types::SourceView<'_>,
        ) -> Result<Vec<RenderOutcome>, OperatingError> {
            Ok(vec![
                RenderOutcome::Known(ryo_language::domain::semantics::types::RenderedNode(Arc::from("a"))),
                RenderOutcome::Unsupported(ryo_relations::domain::terms::types::UnknownReason(24)),
            ])
        }
    }
    let mut case = fixture(OperationKind::RenderSyntax);
    ryo_values::domain::rows::service::push_row(
        &mut case.inputs[0],
        &[Cell::Node(
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
    Ok(())
}
fn a_semantic_batch_of_another_length_is_refused() -> Result<(), Failure> {
    #[derive(Debug)]
    struct OneAnswerTooMany;
    impl ryo_execution_domain::domain::operating::types::SemanticLookup for OneAnswerTooMany {
        fn render_batch(
            &self,
            requests: &[RenderRequest],
            _: ryo_values::domain::files::types::SourceView<'_>,
        ) -> Result<Vec<RenderOutcome>, OperatingError> {
            let known = || {
                RenderOutcome::Known(ryo_language::domain::semantics::types::RenderedNode(
                    Arc::from("a"),
                ))
            };
            Ok((0..=requests.len()).map(|_| known()).collect())
        }
    }
    let observe = |lookup: &dyn ryo_execution_domain::domain::operating::types::SemanticLookup| {
        let mut case = fixture(OperationKind::RenderSyntax);
        ryo_values::domain::rows::service::push_row(
            &mut case.inputs[0],
            &[Cell::Node(ryo_relations::domain::relations::service::node_id(
                ryo_relations::domain::relations::types::FileId(0),
                ryo_relations::domain::relations::types::Position(1),
            ))],
        );
        compare_delivery_with_semantic(
            &case.operation,
            &case.inputs,
            &case.bindings,
            SOURCE,
            &mut case.symbols,
            None,
            Some(lookup),
            RefusalOutcome::Failed,
        )
        .unwrap()
    };
    // Positive control: one answer for each request is published.
    let answered = observe(&KnownSyntax);
    assert!(matches!(&answered.collected, Ok(table) if table.count.0 > 0));
    assert_eq!(answered.stream_error, None);
    assert!(!answered.rows.is_empty());
    // Negative control: one answer more than there are requests is refused, and no row is published.
    let refused = observe(&OneAnswerTooMany);
    assert!(matches!(
        refused.collected,
        Err(OperatingError::SemanticBatchShape)
    ));
    assert!(matches!(
        refused.stream_error,
        Some(OperatingError::SemanticBatchShape)
    ));
    assert!(refused.rows.is_empty());
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(a_semantic_batch_of_another_length_is_refused)),
        word: Word::Succeeds,
        input: || a_semantic_batch_of_another_length_is_refused,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(engine_driver_preserves_first_refusal_and_denies_later_semantic_callbacks)),
        word: Word::Succeeds,
        input: || engine_driver_preserves_first_refusal_and_denies_later_semantic_callbacks,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(every_registered_class_runs_real_collect_and_stream_with_independent_expectations)),
        word: Word::Succeeds,
        input: || every_registered_class_runs_real_collect_and_stream_with_independent_expectations,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(unsupported_semantic_batch_publishes_no_prefix)),
        word: Word::Succeeds,
        input: || unsupported_semantic_batch_publishes_no_prefix,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(every_required_input_empty_answers_no_rows)),
        word: Word::Succeeds,
        input: || every_required_input_empty_answers_no_rows,
        expect: Expect::Value(()),
    },];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
