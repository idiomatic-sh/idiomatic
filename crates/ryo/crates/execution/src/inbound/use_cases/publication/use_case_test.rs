//! The engine's publication carries producer symbols into the caller's arena and refuses the reserved arena untouched.
use super::{interface::PublicationUseCase, use_case::PublicationUseCaseImpl};
use crate::application::publication::{errors::PublicationError, types::PublicationInput};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::inbound::use_cases::publication::interface::PublicationReason;
use ryo_language::domain::symbols::{
    service::intern_derived,
    types::{ByteSlice, Symbols},
};
use ryo_relations::domain::{
    relations::types::{CROSS_FILE, CellKind, Integer, Value as Cell, WorkerNumber},
    terms::types::NameText,
};
use ryo_values::domain::{
    rows::types::{RowCount, Table},
    stored_facts::types::ArenaBytes,
};
use std::borrow::Cow;

/// What a producer publishes: its one cell, and whether its worker is the reserved cross-file one.
#[derive(Clone, Copy, Debug)]
struct Publishing {
    symbol: bool,
    reserved: bool,
}

/// What the publication left: whether the cell became a portable name or stayed as it was, the
/// caller's arena (which held `[7]`) and the number of places written.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Published {
    named: bool,
    unchanged: bool,
    arena: Cow<'static, [u8]>,
    places: usize,
}

/// A refused publication: the error, its message, and what it left.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Refused {
    error: PublicationError,
    message: Cow<'static, str>,
    left: Published,
}

type Input = Publishing;
type Value = Published;
type Failure = Refused;
/// The name the symbol cell interns.
const NAME: &str = "alpha";
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(symbol_gets_portable_text_and_place)),
        word: Word::Succeeds,
        input: || Publishing {
            symbol: true,
            reserved: false,
        },
        expect: Expect::Value(Published {
            named: true,
            unchanged: false,
            arena: Cow::Borrowed(&[7, b'a', b'l', b'p', b'h', b'a']),
            places: 1,
        }),
    },
    Case {
        name: CaseName(stringify!(numeric_row_remains_complete)),
        word: Word::Succeeds,
        input: || Publishing {
            symbol: false,
            reserved: false,
        },
        expect: Expect::Value(Published {
            named: false,
            unchanged: true,
            arena: Cow::Borrowed(&[7]),
            places: 0,
        }),
    },
    Case {
        name: CaseName(stringify!(reserved_arena_refuses_without_mutation)),
        word: Word::Refuses,
        input: || Publishing {
            symbol: false,
            reserved: true,
        },
        expect: Expect::Error(Refused {
            error: PublicationError::Publication {
                reason: PublicationReason::ReservedArena,
            },
            message: Cow::Borrowed(
                "cross publication requires an ordinary arena slot and fresh place log",
            ),
            left: Published {
                named: false,
                unchanged: true,
                arena: Cow::Borrowed(&[7]),
                places: 0,
            },
        }),
    },
];
/// Publishes the producer's cell through the use case, into an arena holding `[7]`.
fn subject(publishing: Input) -> Result<Value, Failure> {
    let mut symbols = Symbols::default();
    let mut table = if publishing.symbol {
        let symbol = intern_derived(&mut symbols, ByteSlice(&[]), NameText(NAME));
        Table {
            kinds: vec![CellKind::Symbol],
            cells: vec![Cell::Symbol(symbol)],
            count: RowCount(1),
        }
    } else {
        Table {
            kinds: vec![CellKind::Number],
            cells: vec![Cell::Number(Integer(17))],
            count: RowCount(1),
        }
    };
    let before = table.cells.clone();
    let worker = if publishing.reserved {
        CROSS_FILE
    } else {
        WorkerNumber(0)
    };
    let mut arena = ArenaBytes(vec![7]);
    let mut places = Vec::new();
    let answer = PublicationUseCaseImpl::new().publication(PublicationInput {
        table: &mut table,
        symbols: &mut symbols,
        inherited: &[],
        worker,
        arena: &mut arena,
        places: &mut places,
    });
    let left = Published {
        named: matches!(table.cells[0], Cell::Name(_)),
        unchanged: table.cells == before,
        arena: Cow::Owned(arena.0),
        places: places.len(),
    };
    match answer {
        Ok(()) => Ok(left),
        Err(error) => Err(Refused {
            message: Cow::Owned(error.to_string()),
            error,
            left,
        }),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
