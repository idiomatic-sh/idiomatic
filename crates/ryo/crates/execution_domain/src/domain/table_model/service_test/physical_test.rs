//! Physical rows of the zero-width and native representations, through the registry.
use crate::domain::operating::errors::OperatingError;
use crate::domain::table_cells::types::{TreeForm, TreeRows};
use crate::domain::table_model::collections::representations::types::*;
use crate::domain::table_model::service::{borrowed, kind, registered};
use crate::domain::table_model::service_test::{check_rows, number, values};
use crate::domain::table_model::types::Rows;
use crate::domain::table_model::types::{PhysicalShape, RepresentationKind};
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::completion::service::{completion, name_kind};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::service::{node_id, to_bits};
use ryo_relations::domain::relations::types::{
    ColumnsView, FileId, KindBits, Offset, Position, Span, Value as Cell,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};

/// A scenario: what a row asks of the part.
type Input = fn() -> Result<Value, Failure>;
/// What a scenario answers: a yes as one.
type Value = usize;
/// Why a scenario is refused.
type Failure = OperatingError;

fn zero_width_population_preserves_each_logical_row_without_inventing_stored_rows()
-> Result<Value, Failure> {
    let table = Table {
        kinds: vec![],
        cells: vec![],
        count: RowCount(3),
    };
    check_rows(&Rows::Computed(&table), &[vec![], vec![], vec![]]);
    let empty = ColumnsView {
        kinds: &[],
        columns: &[],
    };
    check_rows(&Rows::Stored(empty), &[]);
    Ok(1)
}

fn native_elements_and_edges_preserve_utf8_spans_and_do_not_publish_end_sentinels()
-> Result<Value, Failure> {
    let bytes = "é".as_bytes();
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let mut builder = table_builder(&mut elements, &mut symbols, ByteSlice(bytes));
    enter_node(&mut builder, KindBits(0), Offset(0));
    token(&mut builder, KindBits(1), Offset(0), Offset(2));
    leave_node(&mut builder, Offset(2));
    let mut named = completion(&mut elements, &mut symbols, ByteSlice(bytes), FileId(7));
    name_kind(&mut named, KindBits(0), NameText("module"));
    name_kind(&mut named, KindBits(1), NameText("name"));
    let item = registered(RepresentationKind::Tree);
    for form in [TreeForm::Elements, TreeForm::Edges] {
        let input = Rows::Tree(TreeRows {
            elements: &elements,
            file: FileId(7),
            form,
        });
        let raw = borrowed(&input);
        let contract = item.prepare(raw).unwrap();
        assert_eq!(item.inspect(&contract).unwrap(), PhysicalShape::Complete);
        assert_eq!(
            contract.rows,
            RowCount(if form == TreeForm::Elements { 2 } else { 1 })
        );
        let mut decode = [number(0); 5];
        let row = values(item.row(raw, RowIndex(0), &mut decode).unwrap());
        if form == TreeForm::Elements {
            assert_eq!(
                row[4],
                Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(2)
                })
            );
        } else {
            assert_eq!(row[0], Cell::Node(node_id(FileId(7), Position(0))));
            assert_eq!(row[1], Cell::Node(node_id(FileId(7), Position(1))));
        }
        assert!(matches!(
            item.row(raw, RowIndex(contract.rows.0), &mut decode)
                .unwrap(),
            RowRead::MissingPhysical
        ));
    }
    Ok(1)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "zero_width_population_preserves_each_logical_row_without_inventing_stored_rows",
        ),
        word: Word::Succeeds,
        input: || zero_width_population_preserves_each_logical_row_without_inventing_stored_rows,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName(
            "native_elements_and_edges_preserve_utf8_spans_and_do_not_publish_end_sentinels",
        ),
        word: Word::Succeeds,
        input: || native_elements_and_edges_preserve_utf8_spans_and_do_not_publish_end_sentinels,
        expect: Expect::Value(1),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    scenario()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
