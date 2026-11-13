//! Delivery through the owner when its operator port keeps answering after a refusal.
use super::super::super::fixtures as owner_fixtures;
use super::super::conformance::fixtures::{ObservedDelivery, Settlement};
use super::super::conformance::test::compare_delivery_with_port;
use super::super::conformance::test::compare_delivery_with_semantic;
use super::super::conformance::test::same_rows;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::SOURCE;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::expectations::{
    Conformance, conformance,
};
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::values::KnownSyntax;
use ryo_execution_runtime::application::file_dag::fixtures::operator_contract::*;
use ryo_execution_runtime::outbound::operators::interface::OperatorsPort;
use ryo_language::domain::semantics::types::RenderOutcome;
use ryo_language::domain::semantics::types::RenderRequest;
use ryo_relations::domain::relations::types::Value as Cell;
use ryo_values::cases::Case;
use ryo_values::cases::CaseName;
use ryo_values::cases::Expect;
use ryo_values::cases::Word;
use ryo_values::cases::run;
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::query_operators::types::OperationKind;
use std::borrow::Cow;
use std::sync::Arc;

type Input = fn() -> Result<Value, Failure>;
type Value = Observed;
type Failure = OperatingError;
/// What one delivery settled to, collected and streamed.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Delivered {
    settlement: Settlement,
    /// The collected table's row count, 0 when it was refused.
    collected: usize,
    /// The collected refusal, as its debug text (a semantic reason is an owned string, which a row cannot hold).
    refusal: Option<Cow<'static, str>>,
    streamed: usize,
    stream_error: Option<Cow<'static, str>>,
}
impl Delivered {
    fn of(observed: &ObservedDelivery) -> Self {
        Self {
            settlement: observed.settlement,
            collected: observed.collected.as_ref().map_or(0, |table| table.count.0),
            refusal: observed
                .collected
                .as_ref()
                .err()
                .map(|error| format!("{error:?}").into()),
            streamed: observed.rows.len(),
            stream_error: observed
                .stream_error
                .as_ref()
                .map(|error| format!("{error:?}").into()),
        }
    }
}
/// One registered class's delivery.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Class {
    /// The kind of the class fixture's operation.
    fixture: OperationKind,
    delivered: Delivered,
    /// The declared schema is the collected table's.
    schema_is_collected: bool,
    /// The streamed rows are the collected table's ordered bag.
    same_rows: bool,
    /// How the delivered rows compare with the fixture's expected rows.
    conformance: Conformance,
}
#[derive(Debug, PartialEq, Eq)]
enum Observed {
    Driver {
        delivered: Delivered,
        sessions: usize,
    },
    Classes(Cow<'static, [(OperationKind, Class)]>),
    One(Delivered),
    Controls {
        answered: Delivered,
        refused: Delivered,
    },
}
#[derive(Debug, Default)]
struct ContinuesAfterDriverRefusal(std::sync::atomic::AtomicUsize);
impl OperatorsPort for ContinuesAfterDriverRefusal {
    #[expect(
        clippy::wildcard_enum_match_arm,
        reason = "the audit intercepts the union's evaluation; every other operator call goes to the owner's operators unchanged"
    )]
    fn operators(
        &self,
        call: ryo_operators::inbound::use_cases::operators::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::inbound::use_cases::operators::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        use ryo_operators::domain::evaluation::types::EvaluationInput;
        use ryo_operators::domain::set_rows::errors::SetRowsError;
        use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
        use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
        use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
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
            other => owner_fixtures::operators().operators(other),
        }
    }
}
fn engine_driver_preserves_first_refusal_and_denies_later_semantic_callbacks()
-> Result<Value, Failure> {
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
        port.clone(),
    )?;
    Ok(Observed::Driver {
        delivered: Delivered::of(&observed),
        sessions: port.0.load(std::sync::atomic::Ordering::Relaxed),
    })
}
fn every_registered_class_runs_real_collect_and_stream_with_independent_expectations()
-> Result<Value, Failure> {
    let semantic = KnownSyntax;
    let mut classes = vec![];
    for kind in OperationKind::ALL {
        let mut case = fixture(*kind);
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
        )?;
        let class = Class {
            conformance: conformance(&case.expected, &observed.rows, &case.symbols),
            fixture: kind_of(&case.operation),
            delivered: Delivered::of(&observed),
            schema_is_collected: observed.schema.as_ref().ok()
                == observed.collected.as_ref().ok().map(|table| &table.kinds),
            same_rows: observed
                .collected
                .as_ref()
                .is_ok_and(|table| same_rows(table, &observed.rows)),
        };
        classes.push((*kind, class));
    }
    Ok(Observed::Classes(classes.into()))
}
fn unsupported_semantic_batch_publishes_no_prefix() -> Result<Value, Failure> {
    #[derive(Debug)]
    struct PartialSyntax;
    impl crate::inbound::use_cases::execution::interface::SemanticLookup for PartialSyntax {
        fn render_batch(
            &self,
            _: &[RenderRequest],
            _: ryo_values::domain::files::types::SourceView<'_>,
        ) -> Result<Vec<RenderOutcome>, OperatingError> {
            Ok(vec![
                RenderOutcome::Known(ryo_language::domain::semantics::types::RenderedNode(
                    Arc::from("a"),
                )),
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
    )?;
    Ok(Observed::One(Delivered::of(&observed)))
}
fn a_semantic_batch_of_another_length_is_refused() -> Result<Value, Failure> {
    #[derive(Debug)]
    struct OneAnswerTooMany;
    impl crate::inbound::use_cases::execution::interface::SemanticLookup for OneAnswerTooMany {
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
    let observe = |lookup: &dyn crate::inbound::use_cases::execution::interface::SemanticLookup| {
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
        compare_delivery_with_semantic(
            &case.operation,
            &case.inputs,
            &case.bindings,
            SOURCE,
            &mut case.symbols,
            None,
            Some(lookup),
        )
        .map(|observed| Delivered::of(&observed))
    };
    // Positive control: one answer for each request is published; negative control: one answer more is refused.
    Ok(Observed::Controls {
        answered: observe(&KnownSyntax)?,
        refused: observe(&OneAnswerTooMany)?,
    })
}
/// A delivery that completed with `rows` rows, collected and streamed alike.
const fn delivered(rows: usize) -> Delivered {
    Delivered {
        settlement: Settlement::Complete,
        collected: rows,
        refusal: None,
        streamed: rows,
        stream_error: None,
    }
}
/// A delivery refused with `error`, collected and streamed alike, publishing no row.
const fn refused(settlement: Settlement, error: &'static str) -> Delivered {
    Delivered {
        settlement,
        collected: 0,
        refusal: Some(Cow::Borrowed(error)),
        streamed: 0,
        stream_error: Some(Cow::Borrowed(error)),
    }
}
/// A class whose own fixture completed with `rows` rows under its declared schema, streamed as the collected bag,
/// each row the fixture's expected row.
const fn complete(kind: OperationKind, rows: usize) -> (OperationKind, Class) {
    (
        kind,
        Class {
            fixture: kind,
            delivered: delivered(rows),
            schema_is_collected: true,
            same_rows: true,
            conformance: Conformance::Conforms,
        },
    )
}
const CLASSES: &[(OperationKind, Class)] = {
    use OperationKind::*;
    &[
        complete(Replace, 1),
        complete(OrderedMatch, 2),
        complete(Select, 3),
        complete(Project, 2),
        complete(Extend, 2),
        complete(Join, 3),
        complete(Union, 3),
        complete(Intersection, 1),
        complete(Difference, 1),
        complete(Antijoin, 2),
        complete(GroupCount, 2),
        complete(GroupDigest, 2),
        complete(Capture, 1),
        complete(Rank, 1),
        complete(Words, 3),
        complete(Axis, 2),
        complete(Closure, 3),
        complete(Cycles, 2),
        complete(Rows, 1),
        complete(Hosted, 2),
        complete(SpanBetween, 1),
        complete(SourceMatches, 2),
        complete(RenderSyntax, 1),
        complete(FitsRendered, 1),
        complete(SpanBounds, 1),
        complete(OrderedDfs, 2),
    ]
};
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(a_semantic_batch_of_another_length_is_refused)),
        word: Word::Succeeds,
        input: || a_semantic_batch_of_another_length_is_refused,
        expect: Expect::Value(Observed::Controls {
            answered: delivered(2),
            refused: refused(Settlement::Failed, "SemanticBatchShape"),
        }),
    },
    Case {
        name: CaseName(stringify!(
            engine_driver_preserves_first_refusal_and_denies_later_semantic_callbacks
        )),
        word: Word::Succeeds,
        input: || engine_driver_preserves_first_refusal_and_denies_later_semantic_callbacks,
        expect: Expect::Value(Observed::Driver {
            delivered: refused(Settlement::Failed, "WrongBinding"),
            sessions: 2,
        }),
    },
    Case {
        name: CaseName(stringify!(
            every_registered_class_runs_real_collect_and_stream_with_independent_expectations
        )),
        word: Word::Succeeds,
        input: || every_registered_class_runs_real_collect_and_stream_with_independent_expectations,
        expect: Expect::Value(Observed::Classes(Cow::Borrowed(CLASSES))),
    },
    Case {
        name: CaseName(stringify!(unsupported_semantic_batch_publishes_no_prefix)),
        word: Word::Succeeds,
        input: || unsupported_semantic_batch_publishes_no_prefix,
        expect: Expect::Value(Observed::One(refused(
            Settlement::Unknown,
            r#"SemanticIncomplete { reason: "selected language cannot render this syntax (reason 24)" }"#,
        ))),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
