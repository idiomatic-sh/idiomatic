//! Publication policy is enforced by fresh public delivery, never a pre-collected root.
use super::*;
use ryo_values::domain::plan_parts::types::LiteralText;

fn program(engine: &ExecutionUseCaseImpl) -> PreparedExecutionProgram {
    let mut input = authored_request(1);
    input.plans[0].steps = vec![PlanStep {
        scope: StepScope::PerFile,
        operation: Operation::FitsRendered {
            from: Input::Relation(Relation::Function),
            texts: vec![],
            literals: vec![LiteralText("x".into())],
            span: ColumnIndex(3),
            max_line: ryo_values::domain::plans::types::MaxLine(80),
            tab_width: ryo_values::domain::plans::types::TabWidth(4),
        },
    }];
    input.plans[0].emit.step = StepId(0);
    engine.prepare(input).expect("actual FitsRendered program")
}

fn rows(second: (u32, u32)) -> Table {
    let mut table = source();
    let mut first = table.cells.clone();
    first[3] = Value::Span(Span {
        start: Offset(0),
        end: Offset(1),
    });
    let mut next = first.clone();
    next[0] = Value::Node(node_id(FileId(0), Position(3)));
    next[3] = Value::Span(Span {
        start: Offset(second.0),
        end: Offset(second.1),
    });
    table.cells = [first, next].concat();
    table.count = RowCount(2);
    table
}

#[test]
fn semantic_failure_discards_every_after_success_row() {
    for second in [(2, 1), (4, 7)] {
        for forwarded in [true, false] {
            let engine = engine();
            let prepared = program(&engine);
            let corpus = corpus();
            let request = engine.begin(&prepared, &corpus);
            let table = rows(second);
            let elements = ryo_language::domain::elements::types::Elements::default();
            let read = |relation| {
                assert_eq!(relation, Relation::Function);
                Ok(SourceRows::Computed(&table))
            };
            let mut symbols = Symbols::default();
            let intents = [FileRuleIntent::Run {
                diagnostic: if forwarded {
                    DiagnosticDelivery::Forwarded
                } else {
                    DiagnosticDelivery::Collected
                },
                observations: ObservationDemand::NotRequired,
                capture: CaptureIntent::Disabled,
            }];
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
                .expect("open actual file");
            if forwarded {
                let mut exposed = Vec::new();
                let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                    exposed.extend_from_slice(row);
                    Ok(())
                });
                assert!(matches!(result, Err(OperatingError::InvalidSpan)));
                assert!(
                    exposed.is_empty(),
                    "a later semantic refusal discards the entire batch"
                );
            } else {
                let outcome = file.rule(RuleIndex(0), demand(), &mut symbols).unwrap();
                assert!(matches!(
                    outcome.emit,
                    RootOutcome::Failed(OperatingError::InvalidSpan)
                ));
            }
            file.finish().expect("release failed invocation");
            request.finish();
        }
    }
}

#[test]
fn committed_rows_keep_order_and_delivery_refusal_remains_retryable() {
    for (refuse, diagnostic) in [
        (false, DiagnosticDelivery::Forwarded),
        (true, DiagnosticDelivery::Forwarded),
        (false, DiagnosticDelivery::Collected),
    ] {
        let engine = engine();
        let prepared = program(&engine);
        let corpus = corpus();
        let request = engine.begin(&prepared, &corpus);
        let table = rows((1, 2));
        let elements = ryo_language::domain::elements::types::Elements::default();
        let read = |relation| {
            assert_eq!(relation, Relation::Function);
            Ok(SourceRows::Computed(&table))
        };
        let mut symbols = Symbols::default();
        let intents = [FileRuleIntent::Run {
            diagnostic,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }];
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
            .expect("open actual file");
        if diagnostic == DiagnosticDelivery::Collected {
            let RootOutcome::Complete(retained) = file
                .rule(RuleIndex(0), demand(), &mut symbols)
                .unwrap()
                .emit
            else {
                panic!("successful collected publication")
            };
            assert_eq!(retained, table);
            file.finish().expect("release collected invocation");
            request.finish();
            continue;
        }
        let mut exposed = Vec::new();
        let result = file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
            if refuse && !exposed.is_empty() {
                return Err(OperatingError::WrongBinding);
            }
            exposed.extend_from_slice(row);
            Ok(())
        });
        if refuse {
            assert!(matches!(result, Err(OperatingError::WrongBinding)));
            assert_eq!(exposed, table.cells[..table.kinds.len()]);
            let evaluated = file
                .counts(RuleIndex(0))
                .unwrap()
                .fed
                .iter()
                .map(|counts| counts.0)
                .collect::<Vec<_>>();
            exposed.clear();
            file.emit_rows(RuleIndex(0), &mut symbols, &mut |row| {
                exposed.extend_from_slice(row);
                Ok(())
            })
            .expect("retry after delivery refusal");
            assert_eq!(
                file.counts(RuleIndex(0))
                    .unwrap()
                    .fed
                    .iter()
                    .map(|counts| counts.0)
                    .collect::<Vec<_>>(),
                evaluated,
                "retrying a sink must not repeat semantic execution"
            );
        } else {
            result.expect("publish after successful semantics");
        }
        assert_eq!(exposed, table.cells);
        assert!(
            matches!(
                file.rule(RuleIndex(0), demand(), &mut symbols),
                Err(OperatingError::NotRun)
            ),
            "forwarded publication cannot be reopened in collected mode"
        );
        file.finish().expect("release successful invocation");
        request.finish();
    }
}

#[cfg(test)]
mod axis;

#[test]
fn captured_name_frames_do_not_inherit_previous_frame_place_numbers() {
    use ryo_language::domain::symbols::{
        service::{hash_in, intern},
        types::ByteSlice,
    };
    use ryo_relations::domain::relations::types::{Origin, Words, WorkerNumber};
    use ryo_values::domain::{
        plan_parts::types::RepeatedRows, stored_facts::types::Bucket,
        table_words::service::decode_complete,
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
        input.plans[0].steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Project {
                from: Input::Relation(Relation::Function),
                columns: vec![ColumnIndex(1)],
                distinct: RepeatedRows::Kept,
            },
        });
        input.plans[0].frontier = vec![Input::Step(StepId(1))];
        let program = engine.prepare(input).expect("valid symbol frontier");
        let corpus = corpus();
        let request = engine.begin(&program, &corpus);
        let mut symbols = Symbols::default();
        let symbol = if representation == "cross_name" {
            ryo_language::domain::symbols::service::intern_derived(
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
                ryo_language::domain::symbols::service::places_mut(&mut symbols).push(
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
                ryo_language::domain::symbols::service::name_made_across(&symbols, symbol)
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
                    resident_stability: facts::ResidentStability::Stable,
                    file: FileId(0),
                    evaluation: &[],
                    publication: &[],
                    intents: &[FileRuleIntent::Run {
                        diagnostic: DiagnosticDelivery::Collected,
                        observations: ObservationDemand::NotRequired,
                        capture: CaptureIntent::DiagnosticAndObservations,
                    }],
                    source: b"abc",
                    elements: &elements,
                    read: &read,
                    semantic: None,
                    kinds: None,
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
                        arena: &mut arena,
                        places: &mut places,
                        words: &mut words,
                    },
                )
                .expect("capture actual frontier");
            assert_eq!(captures.len(), 1);
            let outcome = captures.into_iter().next().unwrap().outcome;
            use ryo_execution::inbound::use_cases::publication::interface::PublicationReason;
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
