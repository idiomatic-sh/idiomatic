//! The tree item reads the elements and the edges of a syntax tree from the tree itself and refuses every other representation.
use super::super::interface::RepresentationsItem;
use super::item::TreeItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::types::PhysicalShape;
use crate::domain::table_model::types::RepresentationInput;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::types::{FileId, KindBits, Offset};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::{RowCount, Table};

/// The rows the tree item is offered.
#[derive(Clone, Copy, Debug)]
enum Offered {
    /// The rows of one form of the tree of a module holding one name token `é`.
    Tree(TreeForm),
    /// An empty table an earlier step computed.
    Computed,
}

/// What the item answers about the rows it admits: their count and their shape.
#[derive(Debug, PartialEq)]
struct Read {
    rows: RowCount,
    shape: PhysicalShape,
}

type Input = Offered;
type Value = Read;
type Failure = OperatingError;

fn subject(offered: Input) -> Result<Value, Failure> {
    match offered {
        Offered::Tree(form) => {
            let bytes = "é".as_bytes();
            let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
            let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
            enter_node(&mut builder, KindBits(0), Offset(0));
            token(&mut builder, KindBits(1), Offset(0), Offset(2));
            leave_node(&mut builder, Offset(2));
            let mut named = completion(&mut elements, &mut symbols, ByteSlice(bytes), FileId(7));
            name_kind(&mut named, KindBits(0), NameText("module"));
            name_kind(&mut named, KindBits(1), NameText("name"));
            read(RepresentationInput::Tree(TreeRows {
                elements: &elements,
                file: FileId(7),
                form,
            }))
        }
        Offered::Computed => read(RepresentationInput::Computed(&Table::default())),
    }
}

/// Admits the representation in bulk, then reads its contract and its shape.
fn read(offered: RepresentationInput<'_>) -> Result<Read, OperatingError> {
    let raw = TreeItem.bulk(offered)?;
    let contract = TreeItem.prepare(raw)?;
    Ok(Read {
        rows: contract.rows,
        shape: TreeItem.inspect(&contract)?,
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("reads_the_elements_and_the_edges_of_a_tree"),
        word: Word::Succeeds,
        input: || Offered::Tree(TreeForm::Elements),
        expect: Expect::Value(Read {
            rows: RowCount(2),
            shape: PhysicalShape::Complete,
        }),
    },
    Case {
        name: CaseName("reads_the_edges_of_a_tree"),
        word: Word::Succeeds,
        input: || Offered::Tree(TreeForm::Edges),
        expect: Expect::Value(Read {
            rows: RowCount(1),
            shape: PhysicalShape::Complete,
        }),
    },
    Case {
        name: CaseName("refuses_a_computed_table"),
        word: Word::Refuses,
        input: || Offered::Computed,
        expect: Expect::Error(OperatingError::WrongOperation),
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
