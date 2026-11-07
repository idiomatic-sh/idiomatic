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
    regex: Option<&ryo_matching::inbound::use_cases::matching::compiled::CompiledRegex>,
    driver: &mut dyn crate::domain::evaluation::types::InvocationDriver,
    scratch: &mut crate::domain::evaluation::types::EvaluationScratch,
    occurrences: &dyn crate::domain::evaluation::service::Occurrences,
) -> Result<(), PortableOperatorFault> {
    let value = crate::domain::evaluation::service::cell(row, column)?;
    scratch.text.clear();
    if driver.text(value, &mut scratch.text) == crate::domain::evaluation::types::TextFound::NotText
    {
        return Err(PortableOperatorFault::ShapeMismatch);
    }
    let regex = regex.ok_or(PortableOperatorFault::WrongBinding)?;
    let text = scratch.text.as_str();
    replace_compiled(
        &|each| occurrences.matches(regex, text, each),
        CellText(text),
        replacement,
        ReplacedText(&mut scratch.replaced),
    )?;
    let appended = driver.mint(CellText(&scratch.replaced));
    crate::domain::append::service::append(row, std::slice::from_ref(&appended), &mut scratch.row);
    driver.emit(&scratch.row)
}
