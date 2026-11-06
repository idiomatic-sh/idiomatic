//! Literal replacement over a borrowed prepared matcher.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::replace::types::{ReplacedText, ReplacementText};
use ryo_matching::inbound::use_cases::matching::{
    compiled::CompiledRegex, interface::MatchingUseCase, types::SearchText,
    use_case::MatchingUseCaseImpl,
};
use ryo_values::domain::row_texts::types::CellText;
use std::ops::ControlFlow;

pub(crate) fn replace_compiled(
    regex: &CompiledRegex,
    text: CellText<'_>,
    replacement: ReplacementText<'_>,
    replaced: ReplacedText<'_>,
) -> Result<(), PortableOperatorFault> {
    let (text, replacement, output) = (text.0, replacement.0, replaced.0);
    output.clear();
    let mut end = 0;
    let mut failure = None;
    let _flow =
        MatchingUseCaseImpl::new().visit_regex_matches(regex, SearchText(text), &mut |found| {
            let found = found.0;
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
