//! The column join counts matching visits, not population size, even on refusal.
use super::super::super::capability::fixtures::Witness;
use super::super::super::*;
use super::super::fixtures::{plan, stored};
use ryo_language::domain::symbols::service::intern_derived;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::terms::types::NameText;
use std::sync::atomic::Ordering;
#[test]
fn actual_column_join_counts_matching_visits_not_population_size_even_on_refusal() {
    use ryo_operators::domain::equality_join::types::JoinInput;
    use ryo_operators::domain::equality_join::types::JoinRow;
    use ryo_operators::inbound::use_cases::operators::types::OperatorAnswer;
    use ryo_operators::inbound::use_cases::operators::types::OperatorCall;
    use ryo_operators::inbound::use_cases::operators::interface::OperatorsUseCase;
    use ryo_operators::domain::contract::types::PortableOperatorFault;
    #[derive(Debug)]
    struct AfterFirstMatch {
        witness: Arc<Witness>,
        refuse: bool,
    }
    impl OperatorsUseCase for AfterFirstMatch {
        fn operators(
            &self,
            call: OperatorCall<'_>,
        ) -> Result<OperatorAnswer, PortableOperatorFault> {
            match call {
                OperatorCall::EqualityJoin {
                    input:
                        JoinInput::Values {
                            left: JoinRow::Columns(left),
                            left_on,
                            driver,
                            ..
                        },
                } if self.refuse => {
                    self.witness.calls.fetch_add(1, Ordering::Relaxed);
                    self.witness.columns.fetch_add(1, Ordering::Relaxed);
                    let key = left_on
                        .iter()
                        .map(|column| {
                            left.cell(*column)
                                .expect("the actual prepared left key exists")
                        })
                        .collect::<Vec<_>>();
                    assert_eq!(
                        driver
                            .next(&key)
                            .expect("the actual indexed candidate is readable"),
                        ryo_operators::domain::equality_join::types::Advance::Admitted
                    );
                    Err(PortableOperatorFault::ShapeMismatch)
                }
                call => self.witness.operators(call),
            }
        }
    }
    for (key, refuse_after_match, hits) in [(1, false, 4), (99, false, 0), (1, true, 1)] {
        let mut symbols = Symbols::default();
        let name = intern_derived(&mut symbols, ByteSlice(b""), NameText("function"));
        let body = intern_derived(&mut symbols, ByteSlice(b""), NameText("body"));
        let function = stored(
            Relation::Function,
            Table {
                kinds: kinds(Relation::Function).to_vec(),
                count: RowCount(3),
                cells: [key, key, 9]
                    .into_iter()
                    .flat_map(|node| {
                        [
                            Cell::Node(NodeId(node)),
                            Cell::Symbol(name),
                            Cell::File(FileId(0)),
                            Cell::Span(Span {
                                start: Offset(0),
                                end: Offset(1),
                            }),
                        ]
                    })
                    .collect(),
            },
        );
        let right = stored(
            Relation::SemanticChild,
            Table {
                kinds: kinds(Relation::SemanticChild).to_vec(),
                count: RowCount(5),
                cells: [(1, 10), (1, 11), (8, 12), (10, 13), (11, 14)]
                    .into_iter()
                    .enumerate()
                    .flat_map(|(ordinal, (parent, child))| {
                        [
                            Cell::Node(NodeId(parent)),
                            Cell::Node(NodeId(child)),
                            Cell::Number(Integer(ordinal as i64)),
                            Cell::Symbol(body),
                            Cell::File(FileId(0)),
                        ]
                    })
                    .collect(),
            },
        );
        let mut authored = plan();
        let Operation::Project { distinct, .. } = &mut authored.steps[0].operation else {
            unreachable!()
        };
        *distinct = RepeatedRows::Kept;
        let witness = Arc::new(Witness::default());
        let actual = execute(
            authored,
            [function, right],
            &mut symbols,
            b"",
            false,
            Arc::new(AfterFirstMatch {
                witness: witness.clone(),
                refuse: refuse_after_match,
            }),
            Faults::default(),
        )
        .unwrap();
        assert!(
            actual.column_prepared,
            "this proof requires the actual compiled column alternative"
        );
        assert_eq!(
            witness.columns.load(Ordering::Relaxed),
            if refuse_after_match { 1 } else { 3 },
            "the real column capability is called once per left row, stopping on its first refusal"
        );
        assert_eq!(
            actual.fed[1].0[1], hits,
            "count every actual next_match result, including the match before a semantic failure"
        );
        if refuse_after_match {
            assert!(matches!(actual.result, Err(OperatingError::ShapeMismatch)));
        } else {
            let expected = if key == 99 {
                vec![]
            } else {
                [10, 11]
                    .into_iter()
                    .flat_map(|child| {
                        [
                            Cell::Node(NodeId(1)),
                            Cell::File(FileId(0)),
                            Cell::Node(NodeId(child)),
                        ]
                    })
                    .collect()
            };
            assert_eq!(actual.result.unwrap().cells, expected);
        }
    }
}
