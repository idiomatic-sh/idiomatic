//! The cases of the `planning` part: the order of a branch's atoms for each role. A test runs as soon as its terms are given;
//! an access with no key is a branch's or a pipeline's first enumeration, or a product of a pipeline; a pattern that would
//! need one has no program.

use super::service::planned;
use super::types::UnitRole;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::types::{
    AtomRows, FlatAtom, FlatBranch, FlatTerm, FlatVariable, RowsPerFile,
};
use crate::domain::steps::types::TestOperation;
use ryo_relations::domain::relations::types::{Integer, Relation};
use std::collections::HashSet;

const fn variable(number: u32) -> FlatTerm {
    FlatTerm::Variable(FlatVariable(number))
}

/// A row of the syntax errors: its element is the variable `id`, its other cells variables from `id + 1` on.
fn errors(id: u32) -> FlatAtom {
    let cells = (0..7).map(|place| {
        if place == 5 {
            FlatTerm::File
        } else {
            variable(id + place)
        }
    });
    FlatAtom::Rows {
        rows: AtomRows::Relation(Relation::SyntaxError),
        cells: cells.collect(),
        per_file: RowsPerFile::Many,
    }
}

/// A row of the syntax tree whose element is the variable `id`.
fn element(id: u32) -> FlatAtom {
    let cells = [
        variable(id),
        variable(id + 50),
        variable(id + 51),
        FlatTerm::File,
        variable(id + 52),
    ];
    FlatAtom::Rows {
        rows: AtomRows::Relation(Relation::SyntaxTree),
        cells: cells.into(),
        per_file: RowsPerFile::Many,
    }
}

fn less(left: u32, right: i64) -> FlatAtom {
    FlatAtom::Test {
        test: TestOperation::Less,
        left: variable(left),
        right: FlatTerm::Number(Integer(right)),
    }
}

/// The atoms of a branch, the role it is planned for, and the variables given at its start.
type Input = (Vec<FlatAtom>, UnitRole, &'static [u32]);
/// The atoms' places in the order they run.
type Value = &'static [usize];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_branch_starts_from_its_smaller_source_and_a_test_runs_as_soon_as_its_term_is_given",
        ),
        word: Word::Succeeds,
        input: || {
            (
                vec![less(1, 5), element(0), errors(0)],
                UnitRole::Branch,
                &[],
            )
        },
        expect: Expect::Value(&[2, 0, 1]),
    },
    Case {
        name: CaseName("a_pattern_starts_from_its_given_cell"),
        word: Word::Succeeds,
        input: || (vec![element(0), errors(0)], UnitRole::Pattern, &[0]),
        expect: Expect::Value(&[0, 1]),
    },
    Case {
        name: CaseName("a_pipeline_reads_a_source_no_key_reaches_as_a_product"),
        word: Word::Succeeds,
        input: || (vec![errors(0), errors(20)], UnitRole::Pipeline, &[]),
        expect: Expect::Value(&[0, 1]),
    },
    Case {
        name: CaseName("a_branch_has_no_program_when_a_source_after_its_first_has_no_key"),
        word: Word::Refuses,
        input: || (vec![errors(0), errors(20)], UnitRole::Branch, &[]),
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("a_pattern_has_no_program_when_nothing_given_keys_its_source"),
        word: Word::Refuses,
        input: || (vec![errors(0)], UnitRole::Pattern, &[]),
        expect: Expect::Error(()),
    },
];

fn subject((atoms, role, given): Input) -> Result<Vec<usize>, Failure> {
    let branch = FlatBranch {
        atoms,
        row: Vec::new(),
    };
    let given: HashSet<FlatVariable> = given.iter().copied().map(FlatVariable).collect();
    planned(&Drafts::default(), &branch, &given, role)
        .map(|plan| plan.order.into_iter().map(|at| at.0).collect())
        .ok_or(())
}

fn ordered(input: Input) -> Result<Value, Failure> {
    subject(input).map(|order| &*Vec::leak(order))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, ordered);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, ordered);
}
