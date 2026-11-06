//! Real relation, region, observation and member roots, including direct bound capture.
use crate::boundary_fixture::{authored_request, corpus, engine, source};
use ryo_execution::inbound::use_cases::execution::{interface::*, types::*};
use ryo_language::domain::{elements::types::Elements, symbols::types::Symbols};
use ryo_relations::domain::relations::{service::node_id, types::*};
use ryo_values::domain::{
    plan_parts::types::{Input, ObservationBindingId, StepId, StepScope},
    records::types::ObservationReadinessLayout,
    plans::types::{Operation, PlanStep},
    rows::types::RowCount,
    stored_facts::types::{Bucket, Digest},
    observations::types::*,
    table_words::service::decode_complete,
};
use std::cell::Cell;

fn named() -> PreparedRequest {
    let mut authored = authored_request(3);
    let columns = vec![ObservationColumn {
        name: ObservationColumnName("node".into()),
        kind: CellKind::Node,
    }];
    authored.plans[0].observations = vec![PlanObservation {
        name: ObservationName("nodes".into()),
        source: Input::Step(StepId(0)),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    }];
    authored.plans[1].observation_inputs = vec![BoundObservationInput {
        name: ObservationBindingName("nodes".into()),
        columns: columns.clone(),
        scope: StepScope::PerFile,
    }];
    authored.plans[1].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::Union {
            left: Input::BoundObservation(ObservationBindingId(0)),
            right: Input::BoundObservation(ObservationBindingId(0)),
        },
    }];
    // The linker retains exports only when another rule reads them, and its
    // canonical export contract uses Step roots. Keep the per-file union as a
    // real export; a separately demanded cross-file step reads the bound input
    // directly, making that Observation source an actual file frontier.
    authored.plans[1].steps.push(PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Union {
            left: Input::BoundObservation(ObservationBindingId(0)),
            right: Input::BoundObservation(ObservationBindingId(0)),
        },
    });
    authored.plans[1].observations = vec![
        PlanObservation {
            name: ObservationName("forwarded".into()),
            source: Input::Step(StepId(0)),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        },
        PlanObservation {
            name: ObservationName("projected".into()),
            source: Input::Step(StepId(1)),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        },
    ];
    authored.plans[2].observation_inputs = vec![
        BoundObservationInput {
            name: ObservationBindingName("forwarded".into()),
            columns: columns.clone(),
            scope: StepScope::PerFile,
        },
        BoundObservationInput {
            name: ObservationBindingName("projected".into()),
            columns: columns.clone(),
            scope: StepScope::CrossFile,
        },
    ];
    authored.plans[2].steps = vec![PlanStep {
        scope: StepScope::CrossFile,
        operation: Operation::Union {
            left: Input::BoundObservation(ObservationBindingId(0)),
            right: Input::BoundObservation(ObservationBindingId(1)),
        },
    }];
    authored.rules[2].observations = ["forwarded", "projected"]
        .into_iter()
        .map(|name| ObservationBinding {
            name: ObservationBindingName(name.into()),
            columns: columns.clone(),
            producers: vec![ObservationProducer::Output {
                rule: authored.rules[1].id.clone(),
                output: ObservationName(name.into()),
            }],
        })
        .collect();
    authored.rules[1].observations = vec![ObservationBinding {
        name: ObservationBindingName("nodes".into()),
        columns,
        producers: vec![ObservationProducer::Output {
            rule: authored.rules[0].id.clone(),
            output: ObservationName("nodes".into()),
        }],
    }];
    authored.observations =
        ObservationIdentity::Named(ObservationReadinessLayout(Digest([117; 16])));
    authored
}

#[test]
fn direct_bound_root_and_union_share_the_actual_member_before_producer_delivery() {
    for consumer_first in [false, true] {
        let engine = engine();
        let program = engine
            .prepare(named())
            .expect("named producer, alias and union");
        assert_eq!(
            program.frontiers(RuleIndex(1)).unwrap(),
            &[
                Input::BoundObservation(ObservationBindingId(0)),
                Input::Step(StepId(0))
            ]
        );
        assert!(program.captures_observation(RuleIndex(1), 0).unwrap());
        assert!(program.captures_observation(RuleIndex(1), 1).unwrap());
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
        let table = source();
        let reads = Cell::new(0);
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            reads.set(reads.get() + 1);
            Ok(SourceRows::Computed(&table))
        };
        let elements = Elements::default();
        let mut symbols = Symbols::default();
        let run = FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::DiagnosticAndObservations,
        };
        let intents = [run, run, FileRuleIntent::Undemanded];
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &intents,
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
                },
                &mut symbols,
            )
            .unwrap();
        assert_eq!(reads.get(), 0, "binding roots must remain lazy");
        let roots = RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::Required,
        };
        let order = if consumer_first { [1, 0] } else { [0, 1] };
        let expected = vec![Value::Node(node_id(FileId(0), Position(2)))];
        for index in order {
            let output = file.rule(RuleIndex(index), roots, &mut symbols).unwrap();
            let RootOutcome::Complete(table) = output.emit else {
                panic!("complete named root")
            };
            assert_eq!(table.count, RowCount(1));
            assert_eq!(table.cells, expected);
            assert_eq!(output.observations.len(), 1);
            let RootOutcome::Complete(observed) = &output.observations[0].outcome else {
                panic!("complete named union observation")
            };
            assert_eq!(observed.cells, expected);
        }
        let work = file.readiness_counts();
        assert!(
            work.evaluations.0 > 0,
            "named roots must enter the scheduler"
        );
        let read_count = reads.get();
        assert!(read_count > 0);
        // The original direct bound root is captured after ordinary delivery.
        let (mut arena, mut places, mut words) = (vec![], vec![], Words::default());
        let captures = file
            .capture_frontiers(
                RuleIndex(1),
                Bucket(0),
                &mut symbols,
                CaptureContext {
                    worker: WorkerNumber(0),
                    arena: &mut arena,
                    places: &mut places,
                    words: &mut words,
                },
            )
            .unwrap();
        assert_eq!(captures.len(), 2);
        for (index, capture) in captures.iter().enumerate() {
            assert_eq!(
                capture.input,
                ryo_values::domain::plan_parts::types::InputNumber(index)
            );
            let captured = capture
                .outcome
                .as_ref()
                .expect("bound and exported publications");
            let decoded =
                decode_complete(&captured.words, &mut Vec::new()).expect("valid publication words");
            assert_eq!(decoded.count, RowCount(1));
            assert_eq!(decoded.cells, expected);
            file.commit_frontier(RuleIndex(1), capture.input).unwrap();
        }
        assert_eq!(
            reads.get(),
            read_count,
            "alias capture cannot rerun its provider"
        );
        assert_eq!(
            file.readiness_counts(),
            work,
            "capture reuses terminal scheduler work"
        );
        file.finish().unwrap();
        request.finish();
    }
}

#[test]
fn a_refused_named_member_cannot_be_replaced_with_a_typed_empty_or_provider_retry() {
    let engine = engine();
    let program = engine.prepare(named()).unwrap();
    assert_eq!(
        program.frontiers(RuleIndex(1)).unwrap(),
        &[
            Input::BoundObservation(ObservationBindingId(0)),
            Input::Step(StepId(0))
        ]
    );
    assert!(program.captures_observation(RuleIndex(1), 0).unwrap());
    assert!(program.captures_observation(RuleIndex(1), 1).unwrap());
    let corpus = corpus();
    let request = engine.begin(&program, &corpus);
    let read = |_| panic!("refused member cannot acquire a relation");
    let elements = Elements::default();
    let mut symbols = Symbols::default();
    let intents = [
        FileRuleIntent::Refused(ApplicabilityRefusal::SyntaxUndetermined),
        FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::Required,
            capture: CaptureIntent::DiagnosticAndObservations,
        },
        FileRuleIntent::Undemanded,
    ];
    let mut file = request
        .open_file(
            FileInput {
                language: SourceLanguage::Recognized,
                resident_stability: facts::ResidentStability::Stable,
                file: FileId(0),
                evaluation: &[],
                publication: &[],
                intents: &intents,
                source: b"abc",
                elements: &elements,
                read: &read,
                semantic: None,
                kinds: None,
            },
            &mut symbols,
        )
        .unwrap();
    let roots = RuleDemand {
        emit: EmitDemand::Enabled,
        observations: ObservationDemand::Required,
    };
    let first = file.rule(RuleIndex(1), roots, &mut symbols).unwrap();
    assert!(matches!(
        first.emit,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(first.observations.len(), 1);
    assert!(matches!(
        first.observations[0].outcome,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
    ));
    let work = file.readiness_counts();
    let second = file.rule(RuleIndex(1), roots, &mut symbols).unwrap();
    assert!(matches!(
        second.emit,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(second.observations.len(), 1);
    assert!(matches!(
        second.observations[0].outcome,
        RootOutcome::Unknown(OperatingError::ObservationIncomplete { .. })
    ));
    assert_eq!(
        file.readiness_counts(),
        work,
        "missing-member retry cannot create a new admission"
    );
    assert!(matches!(
        file.rule(RuleIndex(0), roots, &mut symbols),
        Err(OperatingError::NotRun)
    ));
    file.finish().unwrap();
    request.finish();
}
