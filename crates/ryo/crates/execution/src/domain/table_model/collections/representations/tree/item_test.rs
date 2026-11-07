//! The tree item reads the elements and the edges of a syntax tree from the tree itself and refuses every other representation.
use super::super::{
    interface::RepresentationsItem,
    types::{CellRead, PhysicalShape, RepresentationInput},
};
use super::item::TreeItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::rows::types::{RowCount, Table};
fn reads_the_elements_and_the_edges_of_a_tree() -> Result<(), OperatingError> {
    let bytes = "é".as_bytes();
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
    enter_node(&mut builder, KindBits(0), Offset(0));
    token(&mut builder, KindBits(1), Offset(0), Offset(2));
    leave_node(&mut builder, Offset(2));
    let mut named = completion(&mut elements, &mut symbols, ByteSlice(bytes), FileId(7));
    name_kind(&mut named, KindBits(0), NameText("module"));
    name_kind(&mut named, KindBits(1), NameText("name"));
    for (form, rows) in [(TreeForm::Elements, RowCount(2)), (TreeForm::Edges, RowCount(1))] {
        let raw = RepresentationInput::Tree(TreeRows {
            elements: &elements,
            file: FileId(7),
            form,
        });
        let contract = TreeItem.prepare(raw)?;
        assert_eq!(contract.rows, rows);
        assert_eq!(TreeItem.inspect(&contract)?, PhysicalShape::Complete);
    }
    Ok(())
}
fn refuses_a_computed_table() -> Result<(), OperatingError> {
    let table = Table::default();
    TreeItem
        .bulk(RepresentationInput::Computed(&table))
        .map(|_| ())
}

use ryo_values::cases::{Case, CaseName, Expect, Word, run};
type Input = fn() -> Result<(), OperatingError>;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_elements_and_the_edges_of_a_tree"),
        word: Word::Succeeds,
        input: || reads_the_elements_and_the_edges_of_a_tree,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("refuses_a_computed_table"),
        word: Word::Refuses,
        input: || refuses_a_computed_table,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
];
fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
