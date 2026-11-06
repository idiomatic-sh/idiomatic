//! Portable publication keeps earlier roots when a later root refuses.
use super::publication::pack_roots;
use crate::{
    domain::crossing::types::ObservedRoot, outbound::publication::interface::PublicationPort,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, NamePlace, Value, WorkerNumber};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        observations::types::ProducerReadiness,
        records::types::{ObservationIndex, ObservationReason},
        rows::types::{RowCount, Table},
        stored_facts::types::ArenaBytes,
    },
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct RefusingPublication(AtomicUsize);
impl PublicationPort for RefusingPublication {
    fn publication(
        &self,
        _table: &mut Table,
        _symbols: &mut Symbols,
        _inherited: &[ArenaBytes],
        _worker: WorkerNumber,
        arena: &mut ArenaBytes,
        _places: &mut Vec<NamePlace>,
    ) -> Result<(), ObservationReason> {
        let call = self.0.fetch_add(1, Ordering::SeqCst);
        arena.0.push(if call == 0 { 8 } else { 9 });
        if call == 0 {
            Ok(())
        } else {
            Err(ObservationReason(Box::<str>::default()))
        }
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    LaterRefusal,
    MissingTable,
    Skipped,
    Partial,
}
type Input = Scenario;
type Answer = usize;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Answer, Failure>] = &[
    Case {
        name: CaseName("partial_publication_keeps_words_without_rewriting_unknown_readiness"),
        word: Word::Succeeds,
        input: || Scenario::Partial,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(stringify!(later_refusal_keeps_only_prior_arena_bytes)),
        word: Word::Succeeds,
        input: || Scenario::LaterRefusal,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(stringify!(complete_without_table_is_unknown)),
        word: Word::Succeeds,
        input: || Scenario::MissingTable,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName(stringify!(skipped_root_is_not_normalized)),
        word: Word::Succeeds,
        input: || Scenario::Skipped,
        expect: Expect::Value(0),
    },
];
fn numeric() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: vec![Value::Number(Integer(1))],
        count: RowCount(1),
    }
}
fn subject(scenario: Input) -> Result<Answer, Failure> {
    let root = |output, table, readiness| ObservedRoot {
        output: ObservationIndex(output),
        table,
        readiness,
    };
    let roots = match scenario {
        Scenario::LaterRefusal => vec![
            root(0, Some(numeric()), ProducerReadiness::Complete),
            root(1, Some(numeric()), ProducerReadiness::Complete),
        ],
        Scenario::MissingTable => vec![root(0, None, ProducerReadiness::Complete)],
        Scenario::Skipped => vec![root(0, Some(numeric()), ProducerReadiness::Skipped)],
        Scenario::Partial => vec![root(
            0,
            Some(numeric()),
            ProducerReadiness::Unknown(ObservationReason("partial physical scope".into())),
        )],
    };
    let port = RefusingPublication(AtomicUsize::new(0));
    let transfer = pack_roots(roots, &mut Symbols::default(), &[], WorkerNumber(0), &port);
    match scenario {
        Scenario::Partial => {
            assert_eq!(transfer.arena.0, [8]);
            assert!(transfer.roots[0].words.is_some());
            assert!(matches!(
                transfer.roots[0].readiness,
                ProducerReadiness::Unknown(_)
            ));
        }
        Scenario::LaterRefusal => {
            assert_eq!(transfer.arena.0, [8]);
            assert_eq!(port.0.load(Ordering::SeqCst), 2);
            assert!(transfer.roots[0].words.is_some());
            assert_eq!(transfer.roots[0].readiness, ProducerReadiness::Complete);
            assert!(transfer.roots[1].words.is_none());
            assert!(matches!(
                transfer.roots[1].readiness,
                ProducerReadiness::Unknown(_)
            ));
        }
        Scenario::MissingTable | Scenario::Skipped => {
            assert!(transfer.arena.0.is_empty());
            assert_eq!(port.0.load(Ordering::SeqCst), 0);
            assert!(transfer.roots[0].words.is_none());
            if matches!(scenario, Scenario::MissingTable) {
                assert!(matches!(
                    transfer.roots[0].readiness,
                    ProducerReadiness::Unknown(_)
                ));
            } else {
                assert_eq!(transfer.roots[0].readiness, ProducerReadiness::Skipped);
            }
        }
    }
    Ok(transfer.arena.0.len())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
