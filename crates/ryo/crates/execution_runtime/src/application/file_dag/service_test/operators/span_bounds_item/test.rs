//! Endpoint projection through actual compiled owner execution.
use super::super::fixtures as compiled;
use super::fixtures::{evaluate, span};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    constants::NO_SPAN,
    types::{FileId, Integer, Relation, Value as CellValue},
};
use ryo_values::domain::{
    files::types::SourceView,
    plan_parts::types::{ColumnIndex, Input as PlanInput, StepId},
    plans::types::Operation,
};
fn checked_bounds_preserve_duplicate_rows_and_order() {
    assert_eq!(
        evaluate(&[span(2, 4), span(2, 4), span(0, 0)]),
        Ok(vec![
            CellValue::Span(span(2, 4)),
            CellValue::Number(Integer(2)),
            CellValue::Number(Integer(4)),
            CellValue::Span(span(2, 4)),
            CellValue::Number(Integer(2)),
            CellValue::Number(Integer(4)),
            CellValue::Span(span(0, 0)),
            CellValue::Number(Integer(0)),
            CellValue::Number(Integer(0)),
        ])
    );
}

fn absent_and_reversed_spans_refuse_before_row_output() {
    for invalid in [NO_SPAN, span(4, 3)] {
        assert_eq!(evaluate(&[invalid]), Err(OperatingError::InvalidSpanBounds));
    }
    assert_eq!(
        evaluate(&[span(0, 7)]),
        Ok(vec![
            CellValue::Span(span(0, 7)),
            CellValue::Number(Integer(0)),
            CellValue::Number(Integer(7)),
        ]),
        "pure endpoint extraction does not claim the source bytes of a cross-file row"
    );
}

fn equal_offsets_from_distinct_files_keep_both_file_identities() {
    let providers = [compiled::Provider {
        relation: Relation::SyntaxTree,
        table: compiled::rows(&[
            compiled::FileSpan {
                file: FileId(1),
                span: span(2, 4),
            },
            compiled::FileSpan {
                file: FileId(2),
                span: span(2, 4),
            },
        ]),
    }];
    let steps = vec![
        compiled::select(Relation::SyntaxTree, &[ColumnIndex(3), ColumnIndex(4)]),
        compiled::step(Operation::SpanBounds {
            from: PlanInput::Step(StepId(0)),
            span: ColumnIndex(1),
        }),
    ];
    let compiled::CompiledRun { result: result, .. } = compiled::run(
        steps,
        &providers,
        SourceView(b""),
        &mut Symbols::default(),
        compiled::Downstream::Collected,
    );
    assert_eq!(
        result.expect("pure cross-file projection").cells,
        vec![
            CellValue::File(FileId(1)),
            CellValue::Span(span(2, 4)),
            CellValue::Number(Integer(2)),
            CellValue::Number(Integer(4)),
            CellValue::File(FileId(2)),
            CellValue::Span(span(2, 4)),
            CellValue::Number(Integer(2)),
            CellValue::Number(Integer(4))
        ]
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    CheckedBoundsPreserveDuplicateRowsAndOrder,
    AbsentAndReversedSpansRefuseBeforeRowOutput,
    EqualOffsetsFromDistinctFilesKeepBothFileIdentities,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("checked_bounds_preserve_duplicate_rows_and_order"),
        word: Word::Succeeds,
        input: || Scenario::CheckedBoundsPreserveDuplicateRowsAndOrder,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("absent_and_reversed_spans_refuse_before_row_output"),
        word: Word::Succeeds,
        input: || Scenario::AbsentAndReversedSpansRefuseBeforeRowOutput,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("equal_offsets_from_distinct_files_keep_both_file_identities"),
        word: Word::Succeeds,
        input: || Scenario::EqualOffsetsFromDistinctFilesKeepBothFileIdentities,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::CheckedBoundsPreserveDuplicateRowsAndOrder => {
            checked_bounds_preserve_duplicate_rows_and_order();
            Ok(())
        }
        Scenario::AbsentAndReversedSpansRefuseBeforeRowOutput => {
            absent_and_reversed_spans_refuse_before_row_output();
            Ok(())
        }
        Scenario::EqualOffsetsFromDistinctFilesKeepBothFileIdentities => {
            equal_offsets_from_distinct_files_keep_both_file_identities();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
