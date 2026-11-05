//! File/span key order preserves exact native span matches through the public owner.
use super::super::super::{fixtures, types::*};
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as Source, JoinColumn, Side, StepId},
        plans::types::Operation,
        rows::types::{RowCount, Table},
    },
};
type Input = bool;
type Value = ();
type Failure = OperatingError;
fn subject(stretch_first: Input) -> Result<Value, Failure> {
    let (elements, mut symbols) = fixtures::tree_with_names();
    let word = ryo_language::domain::symbols::service::intern_derived(
        &mut symbols,
        ryo_language::domain::symbols::types::ByteSlice(b"abcd"),
        ryo_relations::domain::terms::types::NameText("fixture"),
    );
    let mut source = Table {
        kinds: kinds(Relation::Function).to_vec(),
        count: RowCount(0),
        cells: vec![],
    };
    for (start, end) in [(1, 2), (0, 2), (7, 9)] {
        source
            .cells
            .extend(kinds(Relation::Function).iter().map(|kind| match kind {
                CellKind::Node => Cell::Node(node_id(FileId(0), Position(0))),
                CellKind::Symbol => Cell::Symbol(word),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(start),
                    end: Offset(end),
                }),
                CellKind::Number => Cell::Number(Integer(0)),
                CellKind::Flag => Cell::Flag(Flag::Lowered),
            }));
        source.count.0 += 1;
    }
    let columns = if stretch_first {
        vec![ColumnIndex(3), ColumnIndex(2)]
    } else {
        vec![ColumnIndex(2), ColumnIndex(3)]
    };
    let right_on = if stretch_first {
        vec![ColumnIndex(4), ColumnIndex(3)]
    } else {
        vec![ColumnIndex(3), ColumnIndex(4)]
    };
    let plan = fixtures::plan(vec![
        Operation::Select {
            from: Source::Relation(Relation::Function),
            comparisons: vec![],
            kept: columns,
        },
        Operation::Join {
            left: Source::Step(StepId(0)),
            right: Source::Relation(Relation::SyntaxTree),
            left_on: vec![ColumnIndex(0), ColumnIndex(1)],
            right_on,
            output: vec![JoinColumn {
                side: Side::Right,
                column: ColumnIndex(0),
            }],
            filter: vec![],
            view: vec![],
            reach: None,
        },
    ]);
    let read = |relation| {
        assert_eq!(relation, Relation::Function);
        Ok(SourceRows::Computed(&source))
    };
    let out = fixtures::published_from(
        plan,
        vec![],
        vec![],
        b"abcd",
        &elements,
        &mut symbols,
        &read,
    )?;
    assert_eq!(out.count.0, 3);
    assert_eq!(out.kinds, [CellKind::Node]);
    assert_eq!(
        out.cells,
        [2, 3, 0].map(|at| Cell::Node(node_id(FileId(0), Position(at))))
    );
    Ok(())
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("file_then_span_keys_find_covering_elements"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("span_then_file_keys_find_covering_elements"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(()),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
