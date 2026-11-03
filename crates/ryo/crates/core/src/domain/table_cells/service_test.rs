//! The `table_cells` service's cases, on values alone.

use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::Flag;
use ryo_relations::domain::relations::types::{
    FileId, KindBits, Offset, Position, Span, Value as Cell,
};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;

use super::service::read_tree_cells;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_cells::types::{CellsRead, ColumnPlaces, RowIndex, TreeForm, TreeRows};
use crate::domain::table_model::errors::TableModelError;

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, TableModelError>;
/// What a scenario answers: a yes as one, or a count.
type Value = usize;
/// Why a scenario is refused.
type Failure = TableModelError;

/// A row of a tree is decided on the cells asked for, and the others are left as they were.
fn reads_only_the_cells_asked_of_a_tree() -> Value {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(b"abcd"));
    enter_node(&mut builder, KindBits(1), Offset(0));
    token(&mut builder, KindBits(2), Offset(1), Offset(3));
    leave_node(&mut builder, Offset(4));
    let tree = TreeRows {
        elements: &elements,
        file: FileId(3),
        form: TreeForm::Elements,
    };
    let mut scratch = vec![Cell::Flag(Flag::Lowered); 5];
    let read = read_tree_cells(&tree, RowIndex(1), &mut scratch, &ColumnPlaces(vec![4, 0]));
    let beyond = read_tree_cells(&tree, RowIndex(2), &mut scratch, &ColumnPlaces(vec![0]));
    let id = Cell::Node(node_id(FileId(3), Position(1)));
    usize::from(
        read == CellsRead::Read
            && beyond == CellsRead::Missing
            && scratch[0] == id
            && scratch[4]
                == Cell::Span(Span {
                    start: Offset(1),
                    end: Offset(3),
                })
            && scratch[1] == Cell::Flag(Flag::Lowered),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("only_the_cells_asked_of_a_tree_are_read"),
    word: Word::Succeeds,
    input: || || Ok(reads_only_the_cells_asked_of_a_tree()),
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
