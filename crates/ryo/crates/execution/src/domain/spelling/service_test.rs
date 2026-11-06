//! The `spelling` service's cases, on values alone.

use ryo_language::domain::symbols::service::{places_mut, places_of};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Name, NamePlace, Offset, Origin, PlaceNumber, TextHash, Value as Cell,
    WorkerNumber,
};
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::selects;

use super::errors::SpellingError;
use super::service::check_spellings;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

use ryo_values::domain::rows::service::push_row;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Hands over two names that have one hash, spelled as these two texts.
    Colliding(&'static str, &'static str),
}

/// What a row of the cases does.
type Input = Scenario;
/// Whether the check held.
type Value = usize;
type Failure = SpellingError;

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::Colliding(first, second) => {
            let mut cross = Symbols::default();
            let mut arena = Vec::new();
            let mut table = Table {
                kinds: vec![CellKind::Symbol],
                ..Table::default()
            };
            for text in [first, second] {
                let start = u32::try_from(arena.len()).unwrap_or(u32::MAX);
                arena.extend_from_slice(text.as_bytes());
                places_mut(&mut cross).push(NamePlace {
                    origin: Origin::Derived(WorkerNumber(0)),
                    file: FileId(0),
                    start: Offset(start),
                    end: Offset(u32::try_from(arena.len()).unwrap_or(u32::MAX)),
                });
                let name = Name {
                    hash: TextHash(7),
                    place: PlaceNumber(u32::try_from(places_of(&cross).len() - 1).unwrap_or(0)),
                };
                push_row(&mut table, &[Cell::Name(name)]);
            }
            let arenas = vec![ArenaBytes(arena)];
            check_spellings([&table], &cross, &arenas).map(|()| 1)
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("names_with_one_hash_and_one_spelling_are_one_entity"),
        word: Word::Succeeds,
        input: || Scenario::Colliding("bee", "bee"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("names_with_one_hash_and_two_spellings_are_refused"),
        word: Word::Refuses,
        input: || Scenario::Colliding("bee", "bat"),
        expect: Expect::Refusal(|error| {
            selects!(matches!(error, SpellingError::EntityInTwoSources { .. }))
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

use ryo_values::domain::rows::types::Table;
