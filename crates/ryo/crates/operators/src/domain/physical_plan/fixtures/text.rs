//! The text program: the case program followed by one node of each text kind, each reading Defines, the program the catalogs
//! of the served stages are checked over.

use super::{Step, case_program, defines, step};
use ryo_values::domain::plan_parts::types::{ColumnIndex, LiteralText};
use ryo_values::domain::plans::types::Operation;

/// The case program followed by one node of each text kind, each reading Defines: the program the catalogs of the served
/// stages are checked over.
#[must_use]
pub fn text_program() -> Vec<Step> {
    use ryo_values::domain::plan_parts::types::{CapturePattern, ParameterId};
    use ryo_values::domain::query_operators::types::{LineOutput, MaxLine, TabWidth, TextOutput};
    let from = || defines().0;
    let read = |operation: Operation| step(operation, &[defines().1]);
    let literal = |text: &str| LiteralText(text.into());
    let mut steps = case_program();
    steps.extend([
        read(Operation::Capture {
            from: from(),
            path: ColumnIndex(3),
            pattern: CapturePattern("{x}".into()),
        }),
        read(Operation::Rank {
            from: from(),
            column: ColumnIndex(3),
            sets: ParameterId(0),
        }),
        read(Operation::Replace {
            from: from(),
            text: ColumnIndex(3),
            pattern: literal("a"),
            replacement: literal("b"),
        }),
        step(
            Operation::OrderedMatch {
                left: from(),
                right: from(),
                left_on: vec![ColumnIndex(3)],
                right_on: vec![ColumnIndex(3)],
                left_order: ColumnIndex(0),
                right_order: ColumnIndex(0),
                output: Vec::new(),
            },
            &[defines().1, defines().1],
        ),
        read(Operation::Words {
            from: from(),
            text: ColumnIndex(3),
        }),
        read(Operation::SpanBetween {
            from: from(),
            start: ColumnIndex(6),
            end: ColumnIndex(6),
        }),
        read(Operation::SourceMatches {
            from: from(),
            span: ColumnIndex(6),
            base_line: None,
            pattern: literal("a"),
            capture: None,
            capture_text: TextOutput::Omitted,
            capture_line: LineOutput::Omitted,
        }),
        read(Operation::RenderSyntax {
            from: from(),
            node: ColumnIndex(0),
            context: ryo_language::domain::semantics::types::RenderContext::PositionalArgument,
        }),
        read(Operation::FitsRendered {
            from: from(),
            texts: vec![ColumnIndex(3)],
            literals: vec![literal(""), literal("")],
            span: ColumnIndex(6),
            max_line: MaxLine(80),
            tab_width: TabWidth(4),
        }),
        read(Operation::SpanBounds {
            from: from(),
            span: ColumnIndex(6),
        }),
    ]);
    steps
}
