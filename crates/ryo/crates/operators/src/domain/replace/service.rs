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
