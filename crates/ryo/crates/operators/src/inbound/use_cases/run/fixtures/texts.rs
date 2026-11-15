//! The texts the text kinds' cases read, and the rows and faults their runs answer as text.
use super::counting::CountingMatching;
use super::{TableInputs, TextResources, emitted_through};
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::fixtures::{Step, defines_rows, over_defines};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::PlanNodeId;
use ryo_relations::domain::relations::types::{Offset, Relation, Span};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Input as StepInput, LiteralText};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_bindings::types::ParameterBinding;
use ryo_values::domain::query_operators::types::{LineOutput, TextOutput};
use ryo_values::domain::rows::types::Table;
use std::sync::Arc;

/// The file the cases read: `const a = 1;`, then a tab and `ab`.
pub const SOURCE: &[&str] = &["const", " a = ", "1", ";\n", "\t", "ab"];

/// The span of bytes `start..end`.
pub fn span(start: u32, end: u32) -> Span {
    Span {
        start: Offset(start),
        end: Offset(end),
    }
}

/// The relations' absent span.
pub fn absent() -> Span {
    span(u32::MAX, 0)
}

/// A literal text.
pub fn literal(text: &str) -> LiteralText {
    LiteralText(text.into())
}

/// A source search of Defines' span with `pattern`, `capture` and `capture_text`.
pub fn search(pattern: &str, capture: Option<&str>, capture_text: TextOutput) -> Step {
    over_defines(Operation::SourceMatches {
        from: StepInput::Relation(Relation::Defines),
        span: ColumnIndex(6),
        base_line: None,
        pattern: literal(pattern),
        capture: capture.map(literal),
        capture_text,
        capture_line: LineOutput::Omitted,
    })
}

/// A replacement of `column`'s text.
pub fn replace(column: u32, pattern: &str) -> Step {
    over_defines(Operation::Replace {
        from: StepInput::Relation(Relation::Defines),
        text: ColumnIndex(column),
        pattern: literal(pattern),
        replacement: literal("_"),
    })
}

/// The answer of the last of `steps` over Defines rows `rows`, each node reading `bindings`, through `matching`.
pub fn answer(
    steps: &[Step],
    rows: &[(&str, Span)],
    bindings: &[ParameterBinding],
    matching: Arc<CountingMatching>,
    resources: &mut TextResources,
    buffers: &mut FileBuffers,
) -> String {
    let named: Vec<_> = rows
        .iter()
        .map(|(name, at)| (resources.symbol(name), *at))
        .collect();
    let table = defines_rows(&named);
    let inputs = TableInputs::new(&[(Relation::Defines, &table)]);
    let last = PlanNodeId(u32::try_from(steps.len()).unwrap_or(u32::MAX) - 1);
    shown(emitted_through(
        steps,
        &[last],
        (bindings, matching),
        &inputs,
        resources,
        buffers,
    ))
}

/// The emitted tables' rows past Defines' columns, or the fault.
pub fn shown(answer: Result<Vec<Table>, PortableOperatorFault>) -> String {
    match answer {
        Ok(tables) => tables
            .iter()
            .map(|table| {
                let width = table.kinds.len().max(1);
                let rows: Vec<String> = table
                    .cells
                    .chunks(width)
                    .map(|row| format!("{:?}", row.get(7.min(row.len())..).unwrap_or_default()))
                    .collect();
                format!("{} rows {}", table.count.0, rows.join(" "))
            })
            .collect::<Vec<_>>()
            .join("; "),
        Err(fault) => format!("{fault:?}"),
    }
}

/// The answer of `steps` over `rows` on fresh resources and buffers, with no bindings.
pub fn plain(steps: &[Step], rows: &[(&str, Span)]) -> String {
    let mut resources = TextResources::new(SOURCE);
    answer(
        steps,
        rows,
        &[],
        Arc::default(),
        &mut resources,
        &mut FileBuffers::default(),
    )
}
