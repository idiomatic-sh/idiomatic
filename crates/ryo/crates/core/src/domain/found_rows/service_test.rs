//! The `found_rows` service's cases, on values alone: the rows a scan kept render as the cells the old engine's rows of the same
//! values render as; a row past the limit is counted and not rendered; a row that lacks its subject is refused; and two
//! renderings of a file compare by their rows' cells, whatever the engines number a node.

use super::service::{first_difference, render_found};
use super::types::{Compared, FoundInput};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::finish::errors::FinishError;
use crate::domain::finish::types::LineStarts;
use crate::domain::ordering::service::sort_findings;
use crate::domain::render::service::{lent_row, render_rows, row_cells};
use crate::domain::render::types::{
    FindingRule, RenderInput, Rendered, RowLayout, RowLimit, RowNumber, Shown,
};
use crate::domain::value_cells::types::Scene;
use ryo_language::domain::builders::service::{enter_node, leave_node, table_builder, token};
use ryo_language::domain::elements::types::Elements;
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::symbols::service::intern;
use ryo_language::domain::symbols::types::{ByteSlice, Symbols};
use ryo_relations::domain::relations::service::{node_id, to_bits};
use ryo_relations::domain::relations::types::{
    CellKind, FileId, KindBits, Offset, Position, Span, Value as Held,
};
use ryo_values::domain::files::types::SourceView;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Heading, StepId};
use ryo_values::domain::plans::types::Emit;
use ryo_values::domain::row_texts::types::{Cell, RowsTextView};
use ryo_values::domain::rows::service::push_row;
use ryo_values::domain::rows::types::Table;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::runs::types::RuleNumber;
use ryo_values::domain::scan_rows::types::{KeptCell, KeptText, RowPlace, ScanRow};
use std::borrow::Cow;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// The two words of the file as findings: the rows a scan kept, rendered.
    Found,
    /// The same rows as the old engine's table holds them, rendered by the old engine's rendering, and whether the rendering of
    /// the kept rows is the same, cell for cell and byte for byte.
    AsTheOldEngine,
    /// The two rows with a limit of one.
    Limited,
    /// The two renderings compared when the new engine numbers the words' nodes otherwise.
    OtherNodeNumbers,
    /// The two renderings compared when the new engine's second row is one byte shorter.
    OtherSpan,
    /// The two renderings compared when the new engine found one row only.
    OneRowLess,
    /// A row with no file column, whose rule's subject is the file.
    NoSubject,
}

type Input = Scenario;
/// What the case saw, in lines.
type Value = Cow<'static, str>;
type Failure = FinishError;

const SOURCE: &[u8] = b"two\n one\n";

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("kept_rows_render_as_findings_with_their_place"),
        word: Word::Succeeds,
        input: || Scenario::Found,
        expect: Expect::Value(Cow::Borrowed(
            "2 of 2 rows\n'words' file a.walk 1 'a.walk#1' 'id' 'a.walk#1' 'file' file a.walk 'span' 0..3 'text' 'two' 1 1\n'words' file a.walk 1 'a.walk#2' 'id' 'a.walk#2' 'file' file a.walk 'span' 5..8 'text' 'one' 2 2",
        )),
    },
    Case {
        name: CaseName("kept_rows_render_as_the_old_engines_rows_of_the_same_values"),
        word: Word::Succeeds,
        input: || Scenario::AsTheOldEngine,
        expect: Expect::Value(Cow::Borrowed(
            "the same rendering: true\n2 of 2 rows\n'words' file a.walk 1 'a.walk#1' 'id' 'a.walk#1' 'file' file a.walk 'span' 0..3 'text' 'two' 1 1\n'words' file a.walk 1 'a.walk#2' 'id' 'a.walk#2' 'file' file a.walk 'span' 5..8 'text' 'one' 2 2",
        )),
    },
    Case {
        name: CaseName("a_row_past_the_limit_is_counted_and_not_rendered"),
        word: Word::Succeeds,
        input: || Scenario::Limited,
        expect: Expect::Value(Cow::Borrowed(
            "1 of 2 rows\n'words' file a.walk 1 'a.walk#1' 'id' 'a.walk#1' 'file' file a.walk 'span' 0..3 'text' 'two' 1 1",
        )),
    },
    Case {
        name: CaseName("rows_that_differ_only_in_a_nodes_number_compare_equal"),
        word: Word::Succeeds,
        input: || Scenario::OtherNodeNumbers,
        expect: Expect::Value(Cow::Borrowed("None")),
    },
    Case {
        name: CaseName("a_planted_span_is_the_first_row_that_differs"),
        word: Word::Succeeds,
        input: || Scenario::OtherSpan,
        expect: Expect::Value(Cow::Borrowed(
            "Some(RowDifference { rule: RuleNumber(0), old: Some(ComparedRow(\"words\\ta.walk\\t1\\t#\\tid\\t#\\tfile\\ta.walk\\tspan\\t5..8\\ttext\\tone\\t2\\t2\")), new: Some(ComparedRow(\"words\\ta.walk\\t1\\t#\\tid\\t#\\tfile\\ta.walk\\tspan\\t5..7\\ttext\\ton\\t2\\t2\")) })",
        )),
    },
    Case {
        name: CaseName("a_missing_row_is_the_first_row_that_differs"),
        word: Word::Succeeds,
        input: || Scenario::OneRowLess,
        expect: Expect::Value(Cow::Borrowed(
            "Some(RowDifference { rule: RuleNumber(0), old: Some(ComparedRow(\"words\\ta.walk\\t1\\t#\\tid\\t#\\tfile\\ta.walk\\tspan\\t5..8\\ttext\\tone\\t2\\t2\")), new: None })",
        )),
    },
    Case {
        name: CaseName("a_row_that_lacks_its_subject_is_refused"),
        word: Word::Refuses,
        input: || Scenario::NoSubject,
        expect: Expect::Error(FinishError::Table(
            ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: 1 },
        )),
    },
];

/// What the rule of the cases emits: the element, the file, the span and the text; its subject the file, its key the element.
fn emit() -> Emit {
    Emit {
        step: StepId(0),
        subject: ColumnIndex(1),
        key: vec![ColumnIndex(0)],
        headings: ["id", "file", "span", "text"]
            .map(|name| Heading(name.into()))
            .to_vec(),
    }
}

/// What each column of the rule's rows holds.
const KINDS: [CellKind; 4] = [
    CellKind::Node,
    CellKind::File,
    CellKind::Span,
    CellKind::Symbol,
];

/// The words of the file: where each is, and the node the new engine numbers it, `shift` past its place among the words.
fn words(shift: u32) -> [(Span, Position); 2] {
    let span = |start, end| Span {
        start: Offset(start),
        end: Offset(end),
    };
    [
        (span(0, 3), Position(1 + shift)),
        (span(5, 8), Position(2 + shift)),
    ]
}

/// The rows a scan kept of the file's words, the second ending `short` bytes early.
fn kept(shift: u32, short: u32) -> Vec<ScanRow> {
    let number = |value| KeptCell::Number(CellWord(to_bits(value).0));
    words(shift)
        .into_iter()
        .enumerate()
        .map(|(place, (mut span, node))| {
            span.end = Offset(span.end.0 - if place == 1 { short } else { 0 });
            let range = usize::try_from(span.start.0).unwrap_or(0)
                ..usize::try_from(span.end.0).unwrap_or(0);
            let text = String::from_utf8_lossy(SOURCE.get(range).unwrap_or_default());
            ScanRow {
                file: FileId(0),
                rule: SelectedRuleOrdinal(0),
                place: RowPlace::At(span),
                cells: vec![
                    number(Held::Node(node_id(FileId(0), node))),
                    number(Held::File(FileId(0))),
                    number(Held::Span(span)),
                    KeptCell::Text(KeptText(text.into())),
                ],
            }
        })
        .collect()
}

/// A finding's row of rule 0, named `words`.
const FINDING: RowLayout<'static> = RowLayout {
    shown: Shown::Finding(FindingRule("words")),
};

/// The rows a scan kept, rendered as findings of the scene's file, at most `limit` of them.
fn found(rows: &[ScanRow], scene: Scene<'_>, limit: usize) -> Result<Rendered, FinishError> {
    let (emit, mut rendered) = (emit(), Rendered::default());
    render_found(
        FoundInput {
            rule: RuleNumber(0),
            emit: &emit,
            kinds: &KINDS,
            layout: FINDING,
            scene,
            limit: RowLimit(limit),
        },
        rows,
        &mut LineStarts::default(),
        &mut rendered,
    )?;
    sort_findings(&mut rendered);
    Ok(rendered)
}

/// The same words as the old engine's table holds them, rendered by its rendering: the file's tree has a root and a token for
/// each word.
fn old(symbols: &mut Symbols, elements: &mut Elements, paths: &[FilePath]) -> Rendered {
    {
        let mut builder = table_builder(elements, symbols, ByteSlice(SOURCE));
        enter_node(&mut builder, KindBits(1), Offset(0));
        for (span, _) in words(0) {
            token(&mut builder, KindBits(2), span.start, span.end);
        }
        leave_node(&mut builder, Offset(9));
    }
    let mut table = Table {
        kinds: KINDS.to_vec(),
        ..Table::default()
    };
    for (span, node) in words(0) {
        let name = intern(symbols, ByteSlice(SOURCE), span.start, span.end);
        push_row(
            &mut table,
            &[
                Held::Node(node_id(FileId(0), node)),
                Held::File(FileId(0)),
                Held::Span(span),
                Held::Symbol(name),
            ],
        );
    }
    let (emit, mut rendered) = (emit(), Rendered::default());
    let input = RenderInput {
        rule: RuleNumber(0),
        table: &table,
        emit: &emit,
        layout: FINDING,
        scene: scene(symbols, elements, paths),
        limit: RowLimit(usize::MAX),
    };
    let done = render_rows(input, &mut LineStarts::default(), &mut rendered);
    assert_eq!(done, Ok(()), "the old engine's rows render");
    sort_findings(&mut rendered);
    rendered
}

/// The scene of the cases' file.
fn scene<'a>(symbols: &'a Symbols, elements: &'a Elements, paths: &'a [FilePath]) -> Scene<'a> {
    Scene {
        unknown: &[],
        file: FileId(0),
        symbols,
        elements,
        source: SourceView(SOURCE),
        paths,
    }
}

/// A rendering as lines: each row's cells, in the order the rows leave in.
fn lines(rendered: &Rendered) -> String {
    let rows: Vec<String> = rendered
        .order
        .iter()
        .map(|each| {
            let cells = row_cells(&rendered.cells, &rendered.ends, RowNumber(each.row.0));
            let shown: Vec<String> = lent_row(RowsTextView(&rendered.text), cells)
                .map(|cell| match cell {
                    Some(Cell::File(path)) => format!("file {}", path.0),
                    Some(Cell::Text(text)) => format!("'{}'", text.0),
                    Some(Cell::Number(number)) => number.0.to_string(),
                    Some(Cell::Span(span)) => format!("{}..{}", span.start.0, span.end.0),
                    None => "-".to_owned(),
                })
                .collect();
            shown.join(" ")
        })
        .collect();
    format!(
        "{} of {} rows\n{}",
        rows.len(),
        rendered.total.0,
        rows.join("\n")
    )
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let paths = vec![FilePath("a.walk".into())];
    let (nothing, bare) = (Symbols::default(), Elements::default());
    let new_scene = scene(&nothing, &bare, &paths);
    let (mut symbols, mut elements) = (Symbols::default(), Elements::default());
    let emit = emit();
    let compared = |rule: RuleNumber| {
        (rule == RuleNumber(0)).then_some(Compared {
            shown: FINDING.shown,
            emit: &emit,
            kinds: &KINDS,
        })
    };
    let differing = |rows: &[ScanRow]| -> Result<Value, Failure> {
        let (mut names, mut tree) = (Symbols::default(), Elements::default());
        let before = old(&mut names, &mut tree, &paths);
        let after = found(rows, new_scene, usize::MAX)?;
        Ok(Cow::Owned(format!(
            "{:?}",
            first_difference(&before, &after, &compared)
        )))
    };
    match scenario {
        Scenario::Found => Ok(Cow::Owned(lines(&found(
            &kept(0, 0),
            new_scene,
            usize::MAX,
        )?))),
        Scenario::AsTheOldEngine => {
            let before = old(&mut symbols, &mut elements, &paths);
            let after = found(&kept(0, 0), new_scene, usize::MAX)?;
            // What the old engine's rendering leaves in `keys` is its own scratch for choosing the rows it renders.
            let same = before.text == after.text
                && before.cells == after.cells
                && before.ends == after.ends
                && before.order == after.order
                && before.total == after.total;
            Ok(Cow::Owned(format!(
                "the same rendering: {same}\n{}",
                lines(&before)
            )))
        }
        Scenario::Limited => Ok(Cow::Owned(lines(&found(&kept(0, 0), new_scene, 1)?))),
        Scenario::OtherNodeNumbers => differing(&kept(7, 0)),
        Scenario::OtherSpan => differing(&kept(0, 1)),
        Scenario::OneRowLess => differing(kept(0, 0).get(..1).unwrap_or_default()),
        Scenario::NoSubject => {
            let mut rows = kept(0, 0);
            for row in &mut rows {
                row.cells.truncate(1);
            }
            found(&rows, new_scene, usize::MAX).map(|rendered| Cow::Owned(lines(&rendered)))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
