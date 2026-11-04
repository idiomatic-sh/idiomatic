//! The `render` service's cases, on values alone.

use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::elements::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::service::node_id;
use ryo_relations::domain::relations::types::{
    CellKind, FileId, Integer, KindBits, Offset, Position, Span, Value as Cell,
};
use ryo_values::domain::records::types::Cell as Typed;
use ryo_values::domain::records::types::{CellText, RowsTextView};

use super::service::{finding_parts, lent_row, render_rows, row_cells, rows_answer};
use super::types::{
    AnswerCell, AnswerColumns, ColumnPlace, FindingRule, RenderInput, Rendered, RowLayout,
    RowLimit, RowNumber, RuleRow, Shown,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::types::{LineStarts, RuleNumber};
use crate::domain::table_model::service::push_row;

use crate::domain::value_cells::types::Scene;
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Heading, StepId};
use ryo_values::domain::plans::types::{Emit, Plan};
use ryo_values::domain::records::types::RowTotal;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Renders two rows with their place and reads the cells of the second.
    Rendered,
    /// Renders two rows with a limit of one: the first by place is kept, both are counted.
    Limited,
    /// Renders a row whose key is a node and reads the place of the element it names.
    Placed,
    /// Makes the answer of two rules whose headings differ and repeat: every column of each is one of the answer's.
    Answered,
    /// Renders a row of the second of two rules among the answer's columns: its id, its own columns, the others empty.
    AmongOthers,
    /// Renders a finding's row and reads its parts back: each value typed, the key a list of values and each column with its heading.
    Finding,
}

/// What a row of the cases does.
type Input = Scenario;
/// Whether the check held.
type Value = usize;
type Failure = FinishError;

const SOURCE: &[u8] = b"fn a() {}\nfn bee() {}\n";

/// Every cell of a row, then its place.
const ALL: RowLayout<'static> = RowLayout { shown: Shown::All };

/// What rendering every cell of the rows of `table` by rule 0 is, at most `limit` of them.
fn every_cell<'a>(
    table: &'a Table,
    emit: &'a Emit,
    scene: Scene<'a>,
    limit: RowLimit,
) -> RenderInput<'a> {
    RenderInput {
        rule: RuleNumber(0),
        table,
        emit,
        layout: ALL,
        scene,
        limit,
    }
}

fn emit() -> Emit {
    Emit {
        step: StepId(0),
        subject: ColumnIndex(0),
        key: vec![ColumnIndex(2)],
        headings: Vec::new(),
    }
}

fn rows(symbols: &mut Symbols) -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File, CellKind::Symbol, CellKind::Span],
        ..Table::default()
    };
    for (start, end) in [(13, 16), (3, 4)] {
        let name = intern(symbols, ByteSlice(SOURCE), Offset(start), Offset(end));
        let span = Span {
            start: Offset(start),
            end: Offset(end),
        };
        push_row(
            &mut table,
            &[Cell::File(FileId(0)), Cell::Symbol(name), Cell::Span(span)],
        );
    }
    table
}

/// The cells of the row rendered `row`th, lent.
fn lent_cells(rendered: &Rendered, row: RowNumber) -> Vec<Option<Typed<CellText<'_>>>> {
    lent_row(
        RowsTextView(&rendered.text),
        row_cells(&rendered.cells, &rendered.ends, row),
    )
    .collect()
}

/// A file value of `path`.
const fn file(path: &str) -> Typed<CellText<'_>> {
    Typed::File(CellText(path))
}

/// A text value of `text`.
const fn text(text: &str) -> Typed<CellText<'_>> {
    Typed::Text(CellText(text))
}

/// A number value of `number`.
const fn number<'a>(number: i64) -> Typed<CellText<'a>> {
    Typed::Number(Integer(number))
}

/// A span value from `start` to `end`.
const fn span<'a>(start: u32, end: u32) -> Typed<CellText<'a>> {
    Typed::Span(Span {
        start: Offset(start),
        end: Offset(end),
    })
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let paths = vec![FilePath("lib.rs".into())];
    let mut symbols = Symbols::default();
    let table = rows(&mut symbols);
    let elements = Elements::default();
    let scene = Scene {
        unknown: &[],
        file: FileId(0),
        symbols: &symbols,
        elements: &elements,
        source: SourceView(SOURCE),
        paths: &paths,
    };
    let mut starts = LineStarts(Vec::new());
    let mut rendered = Rendered::default();
    match scenario {
        Scenario::Rendered => {
            render_rows(
                every_cell(&table, &emit(), scene, RowLimit(usize::MAX)),
                &mut starts,
                &mut rendered,
            )?;
            Ok(usize::from(
                lent_cells(&rendered, RowNumber(0))
                    == [
                        Some(file("lib.rs")),
                        Some(text("a")),
                        Some(span(3, 4)),
                        Some(number(1)),
                        Some(number(4)),
                    ]
                    && lent_cells(&rendered, RowNumber(1)).get(1) == Some(&Some(text("bee")))
                    && rendered.order.len() == 2,
            ))
        }
        Scenario::Limited => {
            render_rows(
                every_cell(&table, &emit(), scene, RowLimit(1)),
                &mut starts,
                &mut rendered,
            )?;
            Ok(usize::from(
                rendered.order.len() == 1
                    && rendered.total == RowTotal(2)
                    && lent_cells(&rendered, rendered.order[0].row)
                        == [
                            Some(file("lib.rs")),
                            Some(text("a")),
                            Some(span(3, 4)),
                            Some(number(1)),
                            Some(number(4)),
                        ]
                    && rendered.text.as_str() == "lib.rsa",
            ))
        }
        Scenario::Placed => {
            let mut placed_symbols = Symbols::default();
            let mut placed_elements = Elements::default();
            let mut builder =
                table_builder(&mut placed_elements, &mut placed_symbols, ByteSlice(SOURCE));
            enter_node(&mut builder, KindBits(1), Offset(0));
            token(&mut builder, KindBits(2), Offset(13), Offset(16));
            leave_node(&mut builder, Offset(22));
            let placed_scene = Scene {
                unknown: &[],
                file: FileId(0),
                symbols: &placed_symbols,
                elements: &placed_elements,
                source: SourceView(SOURCE),
                paths: &paths,
            };
            let mut placed_table = Table {
                kinds: vec![CellKind::File, CellKind::Node],
                ..Table::default()
            };
            push_row(
                &mut placed_table,
                &[
                    Cell::File(FileId(0)),
                    Cell::Node(node_id(FileId(0), Position(1))),
                ],
            );
            let key = Emit {
                key: vec![ColumnIndex(1)],
                ..emit()
            };
            render_rows(
                every_cell(&placed_table, &key, placed_scene, RowLimit(usize::MAX)),
                &mut starts,
                &mut rendered,
            )?;
            Ok(usize::from(
                rendered.order.len() == 1
                    && rendered.order[0].line.0 == 2
                    && rendered.order[0].column.0 == 4,
            ))
        }
        Scenario::Answered => Ok(answered()),
        Scenario::AmongOthers => among_others(&table, scene),
        Scenario::Finding => finding(&table, scene),
    }
}

/// Whether the second of two rules' row is its id, its own column under its heading, and the others empty, then its place.
fn among_others(table: &Table, scene: Scene<'_>) -> Result<Value, Failure> {
    let (mut starts, mut rendered) = (LineStarts(Vec::new()), Rendered::default());
    let cells = [
        AnswerCell::Empty,
        AnswerCell::Column(ColumnPlace(1)),
        AnswerCell::Empty,
    ];
    let layout = RowLayout {
        shown: Shown::Placed(RuleRow {
            rule: FindingRule("2"),
            cells: &cells,
        }),
    };
    let emitted = emit();
    let input = every_cell(table, &emitted, scene, RowLimit(usize::MAX));
    render_rows(RenderInput { layout, ..input }, &mut starts, &mut rendered)?;
    Ok(usize::from(
        lent_cells(&rendered, RowNumber(0))
            == [
                Some(text("2")),
                None,
                Some(text("a")),
                None,
                Some(number(1)),
                Some(number(4)),
            ],
    ))
}

/// Whether a finding's row reads back as its rule, its subject, its key values and its headed columns, then its place.
fn finding(table: &Table, scene: Scene<'_>) -> Result<Value, Failure> {
    let (mut starts, mut rendered) = (LineStarts(Vec::new()), Rendered::default());
    let headed = Emit {
        key: vec![ColumnIndex(1), ColumnIndex(2)],
        headings: ["file", "name", "span"]
            .iter()
            .map(|name| Heading((*name).into()))
            .collect(),
        ..emit()
    };
    let input = every_cell(table, &headed, scene, RowLimit(1));
    let layout = RowLayout {
        shown: Shown::Finding(FindingRule("r")),
    };
    render_rows(RenderInput { layout, ..input }, &mut starts, &mut rendered)?;
    let cells = lent_cells(&rendered, RowNumber(0));
    let Some(found) = finding_parts(&cells) else {
        return Ok(0);
    };
    Ok(usize::from(
        found.rule.0 == "r"
            && *found.subject == Some(file("lib.rs"))
            && found.key == [Some(text("a")), Some(span(3, 4))]
            && found.columns
                == [
                    Some(text("file")),
                    Some(file("lib.rs")),
                    Some(text("name")),
                    Some(text("a")),
                    Some(text("span")),
                    Some(span(3, 4)),
                ]
            && *found.line == Some(number(1))
            && *found.column == Some(number(4)),
    ))
}

/// Whether one rule's answer is its own columns, and two rules' are the rule, then every heading of each as often as one has it.
fn answered() -> Value {
    let headed = |names: &[&str]| Plan {
        observation_inputs: Vec::new(),
        observations: Vec::new(),
        parameters: Vec::new(),
        steps: Vec::new(),
        emit: Emit {
            headings: names.iter().map(|name| Heading((*name).into())).collect(),
            ..emit()
        },
        frontier: Vec::new(),
        comparison_origins: Vec::new(),
    };
    let one = rows_answer(&[headed(&["file", "name"])], RowTotal(5));
    let many = rows_answer(
        &[headed(&["file", "name", "span"]), headed(&["name", "name"])],
        RowTotal(5),
    );
    let names: Vec<&str> = many.headings.iter().map(|heading| &*heading.0).collect();
    let second = match &many.columns {
        AnswerColumns::Many(cells) => cells.get(1).cloned(),
        AnswerColumns::One => None,
    };
    usize::from(
        one.columns == AnswerColumns::One
            && one.headings.len() == 2
            && names == ["rule", "file", "name", "span", "name"]
            && second
                == Some(vec![
                    AnswerCell::Empty,
                    AnswerCell::Column(ColumnPlace(0)),
                    AnswerCell::Empty,
                    AnswerCell::Column(ColumnPlace(1)),
                ]),
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_is_its_cells_and_its_place"),
        word: Word::Succeeds,
        input: || Scenario::Rendered,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_limit_keeps_the_first_rows_by_place_and_counts_them_all"),
        word: Word::Succeeds,
        input: || Scenario::Limited,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_node_is_placed_where_its_element_starts"),
        word: Word::Succeeds,
        input: || Scenario::Placed,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("the_answer_of_many_rules_has_every_column_of_each"),
        word: Word::Succeeds,
        input: || Scenario::Answered,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_rule_fills_its_own_columns_of_the_answer_and_leaves_the_rest_empty"),
        word: Word::Succeeds,
        input: || Scenario::AmongOthers,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_finding_is_its_rule_subject_key_values_and_headed_columns_typed"),
        word: Word::Succeeds,
        input: || Scenario::Finding,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_values::domain::rows::types::Table;
