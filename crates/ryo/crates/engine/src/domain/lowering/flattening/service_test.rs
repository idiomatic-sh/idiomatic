//! The cases of the `flattening` part: the equalities of a branch resolved in one union-find. A class keeps its variable;
//! a constant it is equal to is one pin of it; two constants for one class hold of no row.

use super::service::{pins_of, unified};
use super::types::{
    AtomRows, EqualTerms, FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile, TextNumber,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Relation;

const fn variable(number: u32) -> FlatTerm {
    FlatTerm::Variable(FlatVariable(number))
}

const fn equal(left: FlatTerm, right: FlatTerm) -> EqualTerms {
    EqualTerms { left, right }
}

const fn text(number: u32) -> FlatTerm {
    FlatTerm::Text(TextNumber(number))
}

/// The pairs made equal in a branch of two atoms over variables 0 to 3, whose row is those variables.
type Input = Vec<EqualTerms>;
/// The row after; the number of atoms after; the number of pins after.
type Value = ([FlatTerm; 4], usize, usize);
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_pair_leaves_the_branch_as_it_is"),
        word: Word::Succeeds,
        input: || vec![],
        expect: Expect::Value(([variable(0), variable(1), variable(2), variable(3)], 2, 0)),
    },
    Case {
        name: CaseName("two_variables_made_equal_are_one_variable_everywhere"),
        word: Word::Succeeds,
        input: || vec![equal(variable(1), variable(2))],
        expect: Expect::Value(([variable(0), variable(2), variable(2), variable(3)], 2, 0)),
    },
    Case {
        name: CaseName("a_chain_of_equalities_is_one_class"),
        word: Word::Succeeds,
        input: || {
            vec![
                equal(variable(0), variable(1)),
                equal(variable(1), variable(2)),
                equal(variable(3), variable(0)),
            ]
        },
        expect: Expect::Value(([variable(2), variable(2), variable(2), variable(2)], 2, 0)),
    },
    Case {
        name: CaseName("a_variable_equal_to_a_constant_keeps_the_variable_and_gains_one_pin"),
        word: Word::Succeeds,
        input: || vec![equal(variable(1), text(7)), equal(text(7), variable(1))],
        expect: Expect::Value(([variable(0), variable(1), variable(2), variable(3)], 3, 1)),
    },
    Case {
        name: CaseName("a_key_stays_a_key_when_one_side_of_it_is_a_constant"),
        word: Word::Succeeds,
        input: || vec![equal(variable(1), text(7)), equal(variable(1), variable(2))],
        expect: Expect::Value(([variable(0), variable(2), variable(2), variable(3)], 3, 1)),
    },
    Case {
        name: CaseName("a_variable_equal_to_the_file_is_the_file"),
        word: Word::Succeeds,
        input: || vec![equal(variable(3), FlatTerm::File)],
        expect: Expect::Value((
            [variable(0), variable(1), variable(2), FlatTerm::File],
            2,
            0,
        )),
    },
    Case {
        name: CaseName("two_constants_for_one_class_hold_of_no_row"),
        word: Word::Refuses,
        input: || {
            vec![
                equal(variable(1), text(7)),
                equal(variable(2), text(8)),
                equal(variable(1), variable(2)),
            ]
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("a_constant_equal_to_the_file_holds_of_no_row"),
        word: Word::Refuses,
        input: || {
            vec![
                equal(variable(1), FlatTerm::File),
                equal(variable(1), text(7)),
            ]
        },
        expect: Expect::Error(()),
    },
];

fn subject(pairs: Input) -> Result<Value, Failure> {
    let rows = |cells: [u32; 2]| FlatAtom::Rows {
        rows: AtomRows::Relation(Relation::SyntaxTree),
        cells: cells.into_iter().map(variable).collect(),
        per_file: RowsPerFile::Many,
    };
    let branch = FlatBranch {
        atoms: vec![rows([0, 1]), rows([2, 3])],
        row: (0..4).map(variable).collect(),
    };
    let unified = unified(branch, &pairs).ok_or(())?;
    let row: [FlatTerm; 4] = unified.row.clone().try_into().map_err(|_| ())?;
    Ok((row, unified.atoms.len(), pins_of(&unified).len()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
