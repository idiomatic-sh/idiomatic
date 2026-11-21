//! The cases of the `costing` part: the access of a source for the cells that stand bound, by the fixed priors. A register
//! among the key is a fan-out; constants alone name a bucket of the file; nothing bound is the file's rows.

use super::service::keyed;
use super::types::{CellState, KeyChoice, PlanEstimate};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::definitions::types::Drafts;
use crate::domain::lowering::flattening::types::{AtomRows, RowsPerFile};
use crate::domain::steps::types::{CostClass, MadeTableId};
use ryo_relations::domain::relations::types::Relation;

use CellState::{Constant, File, Open, Register};

/// The source, whether it holds one row a file, and how each of its cells stands.
type Input = (AtomRows, RowsPerFile, &'static [CellState]);
/// The cells of the key (the cell at place n is the bit n), what bounds the access, and the rows it is taken to give, in
/// 1,024ths.
type Value = (u32, CostClass, u64);
type Failure = ();

const fn row(
    name: &'static str,
    input: fn() -> Input,
    expect: Value,
) -> Case<Input, Value, Failure> {
    Case {
        name: CaseName(name),
        word: Word::Succeeds,
        input,
        expect: Expect::Value(expect),
    }
}

const TREE: AtomRows = AtomRows::Relation(Relation::SyntaxTree);
const CHILD: AtomRows = AtomRows::Relation(Relation::SyntaxChild);
const ERRORS: AtomRows = AtomRows::Relation(Relation::SyntaxError);
const COUNTS: AtomRows = AtomRows::Made(MadeTableId(0));

/// The rows of this part's cases: each value is what a run answered, read and found right.
const CASES: &[Case<Input, Value, Failure>] = &[
    row(
        "the_node_table_with_nothing_bound_is_the_files_elements",
        || (TREE, RowsPerFile::Many, &[Open, Open, Open, File, Open]),
        (0, CostClass::File, 2_097_152),
    ),
    row(
        "an_element_given_is_the_key_of_the_node_table",
        || {
            (
                TREE,
                RowsPerFile::Many,
                &[Register, Constant, Open, File, Open],
            )
        },
        (1, CostClass::Fanout, 1_024),
    ),
    row(
        "a_constant_kind_names_a_bucket_of_the_node_table",
        || (TREE, RowsPerFile::Many, &[Open, Constant, Open, File, Open]),
        (2, CostClass::File, 65_536),
    ),
    row(
        "a_parent_given_reaches_its_children",
        || {
            (
                CHILD,
                RowsPerFile::Many,
                &[Register, Open, Open, Open, File],
            )
        },
        (1, CostClass::Fanout, 4_096),
    ),
    row(
        "a_filled_relation_by_its_own_key",
        || {
            (
                ERRORS,
                RowsPerFile::Many,
                &[Register, Open, Open, Open, Open, File, Open],
            )
        },
        (1, CostClass::Fanout, 2_048),
    ),
    row(
        "a_filled_relation_by_constants_alone_is_a_bucket",
        || {
            (
                ERRORS,
                RowsPerFile::Many,
                &[Open, Constant, Open, Open, Open, File, Open],
            )
        },
        (2, CostClass::File, 2_048),
    ),
    row(
        "every_bound_cell_of_a_filled_relation_is_one_key",
        || {
            (
                ERRORS,
                RowsPerFile::Many,
                &[Register, Constant, Open, Register, Open, File, Open],
            )
        },
        (11, CostClass::Fanout, 2_028),
    ),
    row(
        "a_group_table_by_a_register",
        || (COUNTS, RowsPerFile::Many, &[Register, File, Open]),
        (1, CostClass::Fanout, 10_240),
    ),
    row(
        "a_table_of_one_row_a_file_is_keyed_by_the_file",
        || (COUNTS, RowsPerFile::One, &[File, Open]),
        (0, CostClass::Fanout, 1_024),
    ),
];

fn subject((rows, per_file, states): Input) -> Result<Value, Failure> {
    let KeyChoice { cells, class, rows } = keyed(&Drafts::default(), rows, per_file, states);
    let PlanEstimate(rows) = rows;
    let key = cells.iter().fold(0_u32, |key, cell| key | (1 << cell.0));
    Ok((key, class, rows))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
