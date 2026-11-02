//! Physical byte spans, bag order, zero-width matches and refusal before row emission.

use super::super::interface::OperatorsItem as _;
use super::item::SourceMatchesItem;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::{Emit, Feed, InputPlace, OperatorContext, Room, Supply};
use crate::domain::replay::types::Pipe;
use crate::domain::table_model::service::push_row;
use crate::domain::table_model::types::{RowCount, Rows, Table};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_language::domain::relations::constants::NO_SPAN;
use ryo_language::domain::relations::types::{CellKind, Offset, Relation, Span, Value};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as PlanInput, LiteralText};
use ryo_values::domain::plans::types::{Operation, QueryOperator};

fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}

struct Held<'f>(&'f Table);
impl<'f> Feed<'f> for Held<'f> {
    fn kinds(&self, _: InputPlace) -> Result<&[CellKind], OperatingError> {
        Ok(&self.0.kinds)
    }
    fn streams(&self, _: InputPlace) -> Supply {
        Supply::AtHand
    }
    fn found(&mut self, _: InputPlace, _: RowCount) {}
    fn stream(
        &mut self,
        _: InputPlace,
        context: &mut OperatorContext<'_>,
        emit: &mut dyn Emit,
    ) -> Result<(), OperatingError> {
        for row in self.0.cells.chunks_exact(1) {
            emit.row(context, row)?;
        }
        Ok(())
    }
    fn hold<'t>(
        &mut self,
        _: InputPlace,
        _: &mut OperatorContext<'_>,
        _: &'t mut Table,
    ) -> Result<Rows<'t>, OperatingError>
    where
        'f: 't,
    {
        Ok(Rows::Computed(self.0))
    }
}

fn evaluate(
    source: &'static [u8],
    spans: &[Span],
    pattern: &'static str,
) -> Result<Vec<Span>, OperatingError> {
    let mut input = Table {
        kinds: vec![CellKind::Span],
        ..Table::default()
    };
    for &span in spans {
        push_row(&mut input, &[Value::Span(span)]);
    }
    let operation = Operation::SourceMatches {
        from: PlanInput::Relation(Relation::Function),
        span: ColumnIndex(0),
        base_line: None,
        pattern: LiteralText(pattern.into()),
        capture: None,
        capture_text: false,
        capture_line: false,
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    let computed = SourceMatchesItem.compute(
        &operation,
        &[Rows::Computed(&input)],
        &mut context,
        &mut output,
    );
    let mut streamed = Vec::new();
    let streaming = SourceMatchesItem.stream(
        &operation,
        &mut Held(&input),
        &mut context,
        &mut Room::default(),
        &mut Pipe(|_: &mut OperatorContext<'_>, row: &[Value]| {
            streamed.extend_from_slice(row);
            Ok(())
        }),
    );
    assert_eq!(computed, streaming);
    computed?;
    assert_eq!(streamed, output.cells);
    assert_eq!(output.kinds, vec![CellKind::Span, CellKind::Span]);
    Ok(output
        .cells
        .chunks_exact(2)
        .map(|row| match row[1] {
            Value::Span(span) => span,
            _ => panic!("typed occurrence span required"),
        })
        .collect())
}

#[test]
fn nonzero_offset_utf8_and_repeated_source_text_keep_exact_order_and_bag() {
    assert_eq!(
        evaluate("Zéé!".as_bytes(), &[span(1, 5), span(1, 5)], "é"),
        Ok(vec![span(1, 3), span(3, 5), span(1, 3), span(3, 5)])
    );
    assert_eq!(evaluate(b"abc", &[span(0, 3)], "z"), Ok(vec![]));
}

#[test]
fn zero_width_and_absent_spans_are_explicit() {
    assert_eq!(
        evaluate("éx".as_bytes(), &[span(0, 3)], ""),
        Ok(vec![span(0, 0), span(2, 2), span(3, 3)])
    );
    assert_eq!(
        evaluate(b"abc", &[span(1, 1), NO_SPAN], ""),
        Ok(vec![span(1, 1)])
    );
}

#[test]
fn malformed_or_non_utf8_boundary_spans_refuse_before_their_row_emits() {
    for (source, invalid) in [
        (&b"abc"[..], span(2, 1)),
        (&b"abc"[..], span(0, 4)),
        ("Zé!".as_bytes(), span(2, 3)),
    ] {
        assert_eq!(
            evaluate(source, &[invalid], "é"),
            Err(OperatingError::InvalidSourceMatchSpan)
        );
    }
    assert_eq!(
        evaluate(b"a\xffb", &[span(0, 3)], "a"),
        Err(OperatingError::InvalidSourceMatchSpan),
        "a whole-file match must never invent offsets in lossy-decoded source",
    );
}

#[test]
fn contract() {
    super::super::fixtures::contract::check(QueryOperator::SourceMatches);
}

fn evaluate_lines(
    source: &'static [u8],
    source_span: Span,
    base_line: i64,
    pattern: &'static str,
) -> Result<Vec<(Span, i64)>, OperatingError> {
    let mut input = Table {
        kinds: vec![CellKind::Span, CellKind::Number],
        ..Table::default()
    };
    push_row(
        &mut input,
        &[
            Value::Span(source_span),
            Value::Number(ryo_language::domain::relations::types::Integer(base_line)),
        ],
    );
    let operation = Operation::SourceMatches {
        from: PlanInput::Relation(Relation::Function),
        span: ColumnIndex(0),
        base_line: Some(ColumnIndex(1)),
        pattern: LiteralText(pattern.into()),
        capture: None,
        capture_text: false,
        capture_line: false,
    };
    let mut symbols = Symbols::default();
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    SourceMatchesItem.compute(
        &operation,
        &[Rows::Computed(&input)],
        &mut context,
        &mut output,
    )?;
    Ok(output
        .cells
        .chunks_exact(4)
        .map(|row| match (row[2], row[3]) {
            (Value::Span(span), Value::Number(line)) => (span, line.0),
            _ => panic!("span and physical start line required"),
        })
        .collect())
}

#[test]
fn optional_line_uses_match_start_and_counts_crlf_and_single_endings_once() {
    assert_eq!(
        evaluate_lines(b"a\r\nb\rc\nd", span(0, 8), 1, "\nb|b\\rc|\nd|c"),
        Ok(vec![(span(2, 4), 1), (span(5, 6), 3), (span(6, 8), 3)])
    );
    assert_eq!(
        evaluate_lines(b"a\r\nb", span(0, 4), 1, ""),
        Ok(vec![
            (span(0, 0), 1),
            (span(1, 1), 1),
            (span(2, 2), 1),
            (span(3, 3), 2),
            (span(4, 4), 2)
        ])
    );
}

#[test]
fn optional_line_rejects_zero_and_overflow() {
    assert_eq!(
        evaluate_lines(b"x", span(0, 1), 0, "x"),
        Err(OperatingError::InvalidSourceMatchSpan)
    );
    assert_eq!(
        evaluate_lines(b"a\nb", span(0, 3), i64::MAX, "b"),
        Err(OperatingError::InvalidSourceMatchSpan)
    );
}

fn captured(
    source: &'static [u8],
    spans: &[Span],
    pattern: &'static str,
) -> Result<Vec<(Span, Span, String)>, OperatingError> {
    use ryo_language::domain::symbols::service::{intern_derived, text_of};
    use ryo_language::domain::symbols::types::ByteSlice;
    use ryo_language::domain::terms::types::NameText;
    let mut input = Table {
        kinds: vec![CellKind::Span],
        ..Table::default()
    };
    for &source_span in spans {
        push_row(&mut input, &[Value::Span(source_span)]);
    }
    let operation = Operation::SourceMatches {
        from: PlanInput::Relation(Relation::Function),
        span: ColumnIndex(0),
        base_line: None,
        pattern: LiteralText(pattern.into()),
        capture: Some(LiteralText("piece".into())),
        capture_text: true,
        capture_line: false,
    };
    let mut symbols = Symbols::default();
    // Provider comment text is derived before query operators run. Capturing
    // a source substring must not relabel that existing symbol as source-backed.
    let comment = intern_derived(&mut symbols, ByteSlice(source), NameText(" noqa: W605"));
    let mut scratch = OperatorScratch::default();
    let mut memo = Vec::new();
    let mut context = OperatorContext {
        prepared_kinds: None,
        prepared_matchers: None,
        stored_indexes: None,
        bindings: &[],
        symbols: (&mut symbols).into(),
        source: SourceView(source),
        paths: &[],
        arenas: &[],
        memo: &mut memo,
        scratch: &mut scratch,
    };
    let mut output = Table::default();
    SourceMatchesItem.compute(
        &operation,
        &[Rows::Computed(&input)],
        &mut context,
        &mut output,
    )?;
    assert_eq!(
        text_of(&symbols, ByteSlice(source), comment).map(|text| text.0),
        Some(" noqa: W605")
    );
    assert_eq!(
        output.kinds,
        vec![
            CellKind::Span,
            CellKind::Span,
            CellKind::Span,
            CellKind::Symbol
        ]
    );
    Ok(output
        .cells
        .chunks_exact(4)
        .map(|row| {
            let (Value::Span(whole), Value::Span(piece), Value::Symbol(symbol)) =
                (row[1], row[2], row[3])
            else {
                panic!("typed capture outputs required")
            };
            let text = text_of(&symbols, ByteSlice(source), symbol).expect("captured symbol");
            (whole, piece, text.0.to_string())
        })
        .collect())
}

#[test]
fn named_capture_preserves_offsets_order_bag_and_skips_unmatched_branches() {
    assert_eq!(
        captured(
            "é\\q\\z".as_bytes(),
            &[span(2, 6), span(2, 6)],
            r"(?P<piece>\\.)"
        ),
        Ok(vec![
            (span(2, 4), span(2, 4), "\\q".into()),
            (span(4, 6), span(4, 6), "\\z".into()),
            (span(2, 4), span(2, 4), "\\q".into()),
            (span(4, 6), span(4, 6), "\\z".into()),
        ])
    );
    assert_eq!(
        captured(b"\\qz", &[span(0, 3)], r"(?P<piece>\\q)|z"),
        Ok(vec![(span(0, 2), span(0, 2), "\\q".into())])
    );
    assert_eq!(
        captured("é".as_bytes(), &[span(0, 2)], r"(?P<piece>)"),
        Ok(vec![
            (span(0, 0), span(0, 0), "".into()),
            (span(2, 2), span(2, 2), "".into())
        ])
    );
}
