//! Cases execute the public normalization API with real row and symbol storage.
use super::{
    interface::{PublicationError, PublicationUseCase},
    types::PublicationInput,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CROSS_FILE, CellKind, Integer, Value, WorkerNumber};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run, selects},
    domain::{
        rows::types::{RowCount, Table},
        stored_facts::types::ArenaBytes,
    },
};

#[derive(Clone, Copy, Debug)]
enum Scenario {
    Number,
    Symbol,
    ReservedWorker,
}
const CASES: &[Case<Scenario, (), PublicationError>] = &[
    Case {
        name: CaseName("portable numeric row survives normalization"),
        word: Word::Succeeds,
        input: || Scenario::Number,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("producer symbol becomes a portable name"),
        word: Word::Succeeds,
        input: || Scenario::Symbol,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("reserved arena is refused before publication"),
        word: Word::Refuses,
        input: || Scenario::ReservedWorker,
        expect: Expect::Refusal(|error| {
            selects(matches!(error,
            PublicationError::Publication { reason } if reason.as_ref() == "cross publication requires an ordinary arena slot and fresh place log"))
        }),
    },
];
pub(super) fn check(api: &impl PublicationUseCase, word: Word) {
    run(CASES, word, |scenario| {
        let mut symbols = Symbols::default();
        let cells = if matches!(scenario, Scenario::Symbol) {
            use ryo_language::domain::symbols::{service::intern_derived, types::ByteSlice};
            use ryo_relations::domain::terms::types::NameText;
            vec![Value::Symbol(intern_derived(
                &mut symbols,
                ByteSlice(&[]),
                NameText("alpha"),
            ))]
        } else {
            vec![Value::Number(Integer(17))]
        };
        let mut table = Table {
            kinds: vec![if matches!(scenario, Scenario::Symbol) {
                CellKind::Symbol
            } else {
                CellKind::Number
            }],
            cells: cells.clone(),
            count: RowCount(1),
        };
        let mut arena = ArenaBytes(vec![7]);
        let mut places = Vec::new();
        let worker = match scenario {
            Scenario::Number | Scenario::Symbol => WorkerNumber(0),
            Scenario::ReservedWorker => CROSS_FILE,
        };
        let result = api.normalize(PublicationInput {
            table: &mut table,
            symbols: &mut symbols,
            inherited: &[],
            worker,
            arena: &mut arena,
            places: &mut places,
        });
        if matches!(scenario, Scenario::Symbol) {
            assert!(matches!(table.cells[0], Value::Name(_)));
            assert_eq!(arena.0, b"\x07alpha");
            assert_eq!(places.len(), 1);
            assert_eq!(
                places[0].origin,
                ryo_relations::domain::relations::types::Origin::Derived(WorkerNumber(0))
            );
            assert_eq!((places[0].start.0, places[0].end.0), (1, 6));
        } else {
            assert_eq!(table.cells, cells);
            assert_eq!(arena.0, vec![7]);
            assert!(places.is_empty());
        }
        result
    });
}
