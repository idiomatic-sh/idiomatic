//! The `defines` relation's cases.

use super::super::interface::RelationsItem as _;
use super::item::{Defines, DefinesRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId, Offset, Span};
use crate::domain::declaration_kind::types::DeclarationKind;
use crate::domain::terms::types::Symbol;

/// A row to write: every column holds a number of its own, so that a column moved is a column found.
type Input = ();
/// The numbers the row is stored as, in column order.
type Value = [Bits; 7];
type Failure = ();

fn row() -> DefinesRow {
    DefinesRow {
        id: NodeId(3),
        kind: DeclarationKind::Struct,
        refinement: Symbol(7),
        name: Symbol(11),
        owner: NodeId(13),
        file: FileId(17),
        span: Span {
            start: Offset(19),
            end: Offset(23),
        },
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_row_is_7_numbers_in_column_order"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([
        Bits(3),
        Bits(4_294_901_763),
        Bits(7),
        Bits(11),
        Bits(13),
        Bits(17),
        Bits(81_604_378_647),
    ]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(Defines::words(row()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
