//! Captured name frames keep their own place numbers across publications.
use super::super::*;
use ryo_execution_runtime::application::file_dag::program::file_session::types::CaptureContext;

#[test]
fn captured_name_frames_do_not_inherit_previous_frame_place_numbers() {
    use ryo_language::domain::symbols::{
        fixtures::{hash_in, intern},
        types::ByteSlice,
    };
    use ryo_relations::domain::relations::types::{Origin, Words, WorkerNumber};
    use ryo_values::domain::{
        plan_parts::types::RepeatedRows, stored_facts::types::Bucket,
        table_words::fixtures::decode_complete,
    };
    for representation in [
        "symbol",
        "source_name",
        "cross_name",
        "inherited",
        "missing_range",
        "wrong_hash",
        "other_worker",
        "valid_then_missing",
        "forged_cross",
    ] {
        let engine = engine();
        let mut input = authored_request(1);
        crate::boundary_fixture::plans(&mut input)[0]
            .steps
            .push(PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: Input::Relation(Relation::Function),
                    columns: vec![ColumnIndex(1)],
                    distinct: RepeatedRows::Kept,
                },
            });
        crate::boundary_fixture::plans(&mut input)[0].frontier = vec![Input::Step(StepId(1))];
        let program = engine.prepare(input).expect("valid symbol frontier");
        let corpus = corpus();
        let request = ExecutionUseCaseImpl::new().execution(&program, &corpus);
        let mut symbols = Symbols::default();
        let symbol = if representation == "cross_name" {
            ryo_language::domain::symbols::fixtures::intern_derived(
                &mut symbols,
                ByteSlice(b"abc"),
                ryo_relations::domain::terms::types::NameText("abc"),
            )
        } else {
            intern(&mut symbols, ByteSlice(b"abc"), Offset(0), Offset(3))
        };
        let expected_hash = hash_in(&symbols, symbol).expect("interned hash");
        let mut table = source();
        table.cells[1] = match representation {
            "source_name" | "inherited" | "missing_range" | "wrong_hash" | "other_worker"
            | "valid_then_missing" => {
                ryo_language::domain::symbols::fixtures::places_mut(&mut symbols).push(
                    ryo_relations::domain::relations::types::NamePlace {
                        origin: match representation {
                            "source_name" | "valid_then_missing" => Origin::Source,
                            "other_worker" => Origin::Derived(WorkerNumber(1)),
                            _ => Origin::Derived(WorkerNumber(0)),
                        },
                        file: FileId(0),
                        start: Offset(0),
                        end: Offset(3),
                    },
                );
                Value::Name(ryo_relations::domain::relations::types::Name {
                    hash: expected_hash,
                    place: ryo_relations::domain::relations::types::PlaceNumber(0),
                })
            }
            "forged_cross" => Value::Name(ryo_relations::domain::relations::types::Name {
                hash: expected_hash,
                place: ryo_relations::domain::relations::types::PlaceNumber(
                    ryo_relations::domain::relations::types::CROSS_PLACE | symbol.0,
                ),
            }),
            "cross_name" => Value::Name(
                ryo_language::domain::symbols::fixtures::name_made_across(&symbols, symbol)
                    .expect("derived producer name"),
            ),
            _ => Value::Symbol(symbol),
        };
        if representation == "valid_then_missing" {
            let mut second = table.cells.clone();
            second[1] = Value::Name(ryo_relations::domain::relations::types::Name {
                hash: expected_hash,
                place: ryo_relations::domain::relations::types::PlaceNumber(1),
            });
            table.cells.extend(second);
            table.count = RowCount(2);
        }
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            Ok(SourceRows::Computed(&table))
        };
        let elements = ryo_language::domain::elements::types::Elements::default();
        let mut file = request
            .open_file(
                FileInput {
                    language: SourceLanguage::Recognized,
                    resident_stability: ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    source: ryo_values::domain::files::types::SourceView(b"abc"),
                    elements: &elements,
                    read: &read,
                    semantic: ryo_execution_runtime::application::file_dag::program::file_session::types::SemanticSupport::Absent,
                    kinds: ryo_execution_runtime::application::file_dag::program::file_session::types::KindSupport::Absent,
                },
                &mut symbols,
            )
            .expect("open actual public file owner");
        let initial_arena = match representation {
            "inherited" | "other_worker" => b"abc".to_vec(),
            "wrong_hash" => b"xyz".to_vec(),
            _ => Vec::new(),
        };
        let mut arena = initial_arena.clone();
        let sentinel = ryo_relations::domain::relations::types::NamePlace {
            origin: Origin::Source,
            file: FileId(99),
            start: Offset(1),
            end: Offset(2),
        };
        let mut places = vec![sentinel];
        let mut words = Words::default();
        for ordinal in 0..2 {
            let captures = file
                .capture_frontiers(
                    RuleIndex(0),
                    Bucket(0),
                    &mut symbols,
                    CaptureContext {
                        worker: WorkerNumber(0),
                        arena: ryo_execution::inbound::use_cases::execution::interface::Arena(
                            &mut arena,
                        ),
                        places: &mut places,
                        words: &mut words,
                    },
                )
                .expect("capture actual frontier");
            assert_eq!(captures.len(), 1);
            let outcome = captures.into_iter().next().unwrap().outcome;
            use ryo_execution_domain::domain::finish::types::PublicationReason;
            let refusal = match representation {
                "missing_range" => Some(PublicationReason::InheritedRangeMissing),
                "wrong_hash" => Some(PublicationReason::InheritedTextMismatch),
                "other_worker" => Some(PublicationReason::InheritedArenaUnrepresentable),
                "valid_then_missing" => Some(PublicationReason::PlaceMissing),
                "forged_cross" => Some(PublicationReason::CrossNameMismatch),
                _ => None,
            };
            if let Some(reason) = refusal {
                let Err(OperatingError::ObservationIncomplete { reason: actual }) = outcome else {
                    panic!("{representation} must refuse on attempt {ordinal}");
                };
                assert_eq!(actual, reason.to_string(), "exact publication refusal");
                assert_eq!(
                    arena, initial_arena,
                    "failed capture retains no appended bytes"
                );
                continue;
            }
            let capture = outcome.expect("complete frontier");
            let mut decoded_places = Vec::new();
            let decoded =
                decode_complete(&capture.words, &mut decoded_places).expect("complete frame");
            assert_eq!(decoded.count, RowCount(1));
            let [Value::Name(name)] = decoded.cells.as_slice() else {
                panic!("one escaped name")
            };
            // Name equality compares only hashes; assert physical location separately.
            let physical_identity = (name.hash, name.place.0);
            assert_eq!(physical_identity, (expected_hash, 0), "frame {ordinal}");
            let location = decoded_places.first().expect("frame owns its name place");
            assert_eq!(location.origin, Origin::Derived(WorkerNumber(0)));
            assert_eq!(
                &arena[location.start.0 as usize..location.end.0 as usize],
                b"abc"
            );
        }
        assert_eq!(
            places,
            vec![sentinel],
            "capture leaves caller's place log intact"
        );
        file.finish().expect("finish actual file owner");
        request.finish();
    }
}
