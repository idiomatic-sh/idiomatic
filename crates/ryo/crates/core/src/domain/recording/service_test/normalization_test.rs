//! Packed publication normalization, framing and ownership controls.
use super::*;
/// Compare packed publication with the independent word-level remapping contract.
fn normalized_words(
    worker: &Tables,
    file: FileId,
    base: ArenaBase,
) -> Option<Vec<(Bucket, Words)>> {
    use crate::domain::keeping::service::remap_packed;
    use crate::domain::keeping::types::{ArenaStart, Remap};
    let map = Remap {
        from_file: file,
        to_file: FileId(0),
        from_worker: worker.number,
        to_worker: WorkerNumber(0),
        from_base: ArenaStart(base.0),
        to_base: ArenaStart(0),
    };
    worker
        .captured
        .iter()
        .map(|each| {
            let mut words = Words::default();
            remap_packed(&each.words.0, &map, &mut words)?;
            Some((each.bucket, words))
        })
        .collect()
}

fn prepared_packed_buckets_preserve_flat_digests_store_bytes_and_move_ownership() {
    use ryo_relations::domain::relations::service::node_id;
    use ryo_relations::domain::relations::types::{
        Flag, Name, NamePlace, Offset, Origin, PlaceNumber, Position, Span, TextHash,
    };
    let mut worker = tables();
    worker.arena.0 = b"prefix_derived text".to_vec();
    let table = Table {
        kinds: vec![
            CellKind::Node,
            CellKind::Symbol,
            CellKind::File,
            CellKind::Span,
            CellKind::Number,
            CellKind::Flag,
        ],
        cells: vec![
            Cell::Node(node_id(FileId(3), Position(11))),
            Cell::Name(Name {
                hash: TextHash(99),
                place: PlaceNumber(0),
            }),
            Cell::File(FileId(3)),
            Cell::Span(Span {
                start: Offset(1),
                end: Offset(4),
            }),
            Cell::Number(Integer(-1)),
            Cell::Flag(Flag::Raised),
        ],
        count: RowCount(1),
    };
    let places = [NamePlace {
        origin: Origin::Derived(WorkerNumber(2)),
        file: FileId(3),
        start: Offset(8),
        end: Offset(12),
    }];
    let mut complex = Words::default();
    encode_table(&table, &places, &mut complex);
    encode_table(&table, &places, &mut complex);
    let mut zero_width = Words::default();
    encode_table(
        &Table {
            count: RowCount(3),
            ..Table::default()
        },
        &[],
        &mut zero_width,
    );
    // Two explicit producer identities: rule 0 starts at bucket 1 with two
    // frontiers; rule 1 starts at bucket 9 with one. Their fixture digests
    // remain attached while the packed representation is normalized.
    worker.captured = vec![
        Kept {
            bucket: Bucket(9),
            proof: ryo_values::domain::records::types::FrontierProof {
                rule: 1,
                input: 0,
                rule_digest: Digest([18; 16]),
                coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
            },
            words: KeptWords(pack(WordSlice(&complex.0))),
        },
        Kept {
            bucket: Bucket(1),
            proof: ryo_values::domain::records::types::FrontierProof {
                rule: 0,
                input: 0,
                rule_digest: Digest([17; 16]),
                coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
            },
            words: KeptWords(pack(WordSlice(&zero_width.0))),
        },
        Kept {
            bucket: Bucket(2),
            proof: ryo_values::domain::records::types::FrontierProof {
                rule: 0,
                input: 1,
                rule_digest: Digest([17; 16]),
                coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
            },
            words: KeptWords(pack(WordSlice(&[]))),
        },
        Kept {
            bucket: Bucket(9),
            proof: ryo_values::domain::records::types::FrontierProof {
                rule: 1,
                input: 0,
                rule_digest: Digest([18; 16]),
                coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
            },
            words: KeptWords(pack(WordSlice(&words_of_numbers().0))),
        },
    ];
    let expected_words =
        normalized_words(&worker, FileId(3), ArenaBase(7)).expect("reference remap");
    let moved = moved_words(&worker, FileId(3), ArenaBase(7)).expect("incremental remap");
    assert_eq!(moved.len(), expected_words.len());
    for (index, (prepared, (bucket, words))) in moved.iter().zip(&expected_words).enumerate() {
        assert_eq!(prepared.bucket, *bucket);
        assert_eq!(prepared.proof, worker.captured[index].proof);
        assert_eq!(prepared.word_count, words.0.len());
        assert_eq!(prepared.words, pack(WordSlice(&words.0)));
        assert_eq!(prepared.words.0.capacity(), prepared.words.0.len());
    }
    let placed = placed().expect("placement");
    let mut first_plan = plan_with_a_frontier();
    first_plan.frontier.push(PlanInput::Step(StepId(1)));
    let plans = [first_plan, plan_with_a_frontier()];
    let first = [Bucket(1), Bucket(9)];
    let setting = VisitSetting {
        metadata: &from_plans(&plans),
        first_bucket: &first,
        ..setting(&placed)
    };
    let mut flat = Flat(vec![42]);
    for rule in 0..plans.len() {
        let mut expected = Vec::new();
        for (bucket, words) in &expected_words {
            if bucket.0 >= first[rule].0
                && (bucket.0 - first[rule].0) < plans[rule].frontier.len() as u32
            {
                expected.extend([u64::from(bucket.0), words.0.len() as u64]);
                expected.extend_from_slice(&words.0);
            }
        }
        frontier_flat(&moved, None, &[], &setting, RuleIndex(rule), &mut flat);
        assert_eq!(flat.0, expected, "digest framing and captured order");
        assert_eq!(
            blake3::hash(
                &flat
                    .0
                    .iter()
                    .flat_map(|word| word.to_le_bytes())
                    .collect::<Vec<_>>()
            ),
            blake3::hash(
                &expected
                    .iter()
                    .flat_map(|word| word.to_le_bytes())
                    .collect::<Vec<_>>()
            )
        );
    }
    let pointers: Vec<_> = moved.iter().map(|bucket| bucket.words.0.as_ptr()).collect();
    let ran = made_ran(&worker, FileId(3), Digest([1; 16]), moved, ArenaBase(7));
    let ryo_values::domain::records::types::Handing::Made(handed) = ran.handed else {
        panic!("fresh handover");
    };
    for (index, bucket) in handed.buckets.iter().enumerate() {
        let ryo_values::domain::records::types::RecordWords::Inline(words) = &bucket.words else {
            panic!("pure normalization stays inline");
        };
        assert_eq!(
            words.0.as_ptr(),
            pointers[index],
            "no second packing/copy in made_ran"
        );
        assert_eq!(*words, pack(WordSlice(&expected_words[index].1.0)));
        assert_eq!(bucket.proof, worker.captured[index].proof);
    }
    assert_eq!(handed.text.0, b"derived text");
}

fn incremental_normalization_preserves_every_truncated_complete_prefix() {
    let mut worker = tables();
    let mut table = Table {
        kinds: vec![CellKind::Number],
        ..Table::default()
    };
    push_row(&mut table, &[Cell::Number(Integer(-1))]);
    let mut words = Words::default();
    encode_table(&table, &[], &mut words);
    encode_table(&table, &[], &mut words);
    let packed = pack(WordSlice(&words.0));
    for end in 0..=packed.0.len() {
        worker.captured = vec![Kept {
            bucket: Bucket(0),
            proof: ryo_values::domain::records::types::FrontierProof {
                rule: 0,
                input: 0,
                rule_digest: Digest([17; 16]),
                coverage: if end == packed.0.len() {
                    ryo_values::domain::records::types::FrontierCoverage::Complete
                } else {
                    ryo_values::domain::records::types::FrontierCoverage::Partial(vec![
                        ryo_values::domain::records::types::FrontierAffected::CurrentFile,
                    ])
                },
            },
            words: KeptWords(ryo_values::domain::stored_facts::types::PackedWords(
                packed.0[..end].to_vec(),
            )),
        }];
        let expected =
            normalized_words(&worker, FileId(3), ArenaBase(0)).expect("complete normalized prefix");
        let actual = moved_words(&worker, FileId(3), ArenaBase(0)).expect("new valid prefix");
        assert_eq!(actual[0].proof, worker.captured[0].proof);
        assert_eq!(actual[0].word_count, expected[0].1.0.len());
        assert_eq!(
            actual[0].words,
            pack(WordSlice(&expected[0].1.0)),
            "cut byte {end}"
        );
    }
}

fn invalid_last_bucket_is_rejected_for_file_worker_and_arena_underflow() {
    use ryo_relations::domain::relations::types::{
        Name, NamePlace, Offset, Origin, PlaceNumber, TextHash,
    };
    let mut worker = tables();
    let good = Kept {
        bucket: Bucket(0),
        proof: ryo_values::domain::records::types::FrontierProof {
            rule: 0,
            input: 0,
            rule_digest: Digest([17; 16]),
            coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
        },
        words: KeptWords(pack(WordSlice(&words_of_numbers().0))),
    };
    for (value, places) in [
        (Cell::File(FileId(99)), vec![]),
        (
            Cell::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            vec![NamePlace {
                origin: Origin::Derived(WorkerNumber(99)),
                file: FileId(3),
                start: Offset(9),
                end: Offset(12),
            }],
        ),
        (
            Cell::Name(Name {
                hash: TextHash(1),
                place: PlaceNumber(0),
            }),
            vec![NamePlace {
                origin: Origin::Derived(WorkerNumber(2)),
                file: FileId(3),
                start: Offset(2),
                end: Offset(4),
            }],
        ),
    ] {
        let mut words = Words::default();
        encode_table(
            &Table {
                kinds: vec![CellKind::Number],
                cells: vec![value],
                count: RowCount(1),
            },
            &places,
            &mut words,
        );
        worker.captured = vec![
            good.clone(),
            Kept {
                bucket: Bucket(1),
                proof: ryo_values::domain::records::types::FrontierProof {
                    rule: 0,
                    input: 1,
                    rule_digest: Digest([17; 16]),
                    coverage: ryo_values::domain::records::types::FrontierCoverage::Complete,
                },
                words: KeptWords(pack(WordSlice(&words.0))),
            },
        ];
        assert!(normalized_words(&worker, FileId(3), ArenaBase(7)).is_none());
        assert!(moved_words(&worker, FileId(3), ArenaBase(7)).is_none());
    }
}

type Input = fn();
type Value = ();
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "prepared_packed_buckets_preserve_flat_digests_store_bytes_and_move_ownership",
        ),
        word: Word::Succeeds,
        input: || prepared_packed_buckets_preserve_flat_digests_store_bytes_and_move_ownership,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("incremental_normalization_preserves_every_truncated_complete_prefix"),
        word: Word::Succeeds,
        input: || incremental_normalization_preserves_every_truncated_complete_prefix,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("invalid_last_bucket_is_rejected_for_file_worker_and_arena_underflow"),
        word: Word::Succeeds,
        input: || invalid_last_bucket_is_rejected_for_file_worker_and_arena_underflow,
        expect: Expect::Value(()),
    },
];
fn subject(check: fn()) -> Result<(), ()> {
    check();
    Ok(())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
