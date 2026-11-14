//! Literal replacement over a borrowed prepared matcher.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::replace::types::{ReplacedText, ReplacementText};
use ryo_values::domain::row_texts::types::CellText;
use std::ops::ControlFlow;

/// Visits the occurrences of the replaced pattern in the text, stopping when the callback breaks.
pub(crate) type VisitMatches<'visit> =
    &'visit dyn Fn(&mut dyn FnMut(std::ops::Range<usize>) -> ControlFlow<()>) -> ControlFlow<()>;

pub(crate) fn replace_compiled(
    visit: VisitMatches<'_>,
    text: CellText<'_>,
    replacement: ReplacementText<'_>,
    replaced: ReplacedText<'_>,
) -> Result<(), PortableOperatorFault> {
    let (text, replacement, output) = (text.0, replacement.0, replaced.0);
    output.clear();
    let mut end = 0;
    let mut failure = None;
    let _flow = visit(&mut |found| {
        let Some(prefix) = text.get(end..found.start) else {
            failure = Some(PortableOperatorFault::ShapeMismatch);
            return ControlFlow::Break(());
        };
        output.push_str(prefix);
        output.push_str(replacement);
        end = found.end;
        ControlFlow::Continue(())
    });
    if let Some(error) = failure {
        return Err(error);
    }
    output.push_str(
        text.get(end..)
            .ok_or(PortableOperatorFault::ShapeMismatch)?,
    );
    Ok(())
}

/// Appends the text of a scheduled row's column with every match of the acquired matcher replaced by the
/// authored text, interned as derived text; a cell that is not text is refused.
pub(crate) fn evaluate(
    row: &[ryo_relations::domain::relations::types::Value],
    column: ryo_values::domain::plan_parts::types::ColumnIndex,
    replacement: ReplacementText<'_>,
    regex: Option<&ryo_matching::inbound::use_cases::matching::types::CompiledRegex>,
    driver: &mut dyn crate::domain::contract::types::RowDriver,
    scratch: &mut crate::domain::contract::types::RowScratch,
    occurrences: &dyn crate::domain::contract::types::Occurrences,
) -> Result<(), PortableOperatorFault> {
    let value = ryo_values::domain::rows::service::column_in(row, column)
        .copied()
        .ok_or(
            crate::domain::contract::types::PortableOperatorFault::NoSuchColumn { index: column },
        )?;
    scratch.text.0.clear();
    if driver.text(value, &mut scratch.text) == crate::domain::contract::types::TextFound::NotText {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    let regex = regex.ok_or(PortableOperatorFault::WrongBinding)?;
    let text = scratch.text.0.as_str();
    replace_compiled(
        &|each| {
            occurrences.matches(
                regex,
                ryo_matching::inbound::use_cases::matching::types::SearchText(text),
                &mut |found| each(found.0),
            )
        },
        CellText(text),
        replacement,
        ReplacedText(&mut scratch.replaced.0),
    )?;
    let appended = driver.mint(CellText(&scratch.replaced.0));
    crate::domain::append::service::append(row, std::slice::from_ref(&appended), &mut scratch.row);
    driver.emit(&scratch.row)
}

/// The one lowering of a replacement: its text column, pattern and replacement, for the `text_map` kernel.
pub(crate) fn lower(
    text: ryo_values::domain::plan_parts::types::ColumnIndex,
    pattern: &ryo_values::domain::plan_parts::types::LiteralText,
    replacement: &ryo_values::domain::plan_parts::types::LiteralText,
) -> super::types::ReplaceParts {
    super::types::ReplaceParts {
        text,
        pattern: pattern.clone(),
        replacement: replacement.clone(),
    }
}
