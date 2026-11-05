//! Literal replacement over a borrowed prepared matcher.
use crate::domain::contract::types::PortableOperatorFault;
use ryo_matching::inbound::use_cases::matching::{
    compiled::CompiledRegex, interface::MatchingUseCase, use_case::MatchingUseCaseImpl,
};
use std::ops::ControlFlow;

pub(crate) fn replace_compiled(
    regex: &CompiledRegex,
    text: &str,
    replacement: &str,
    output: &mut String,
) -> Result<(), PortableOperatorFault> {
    output.clear();
    let mut end = 0;
    let mut failure = None;
    let _flow = MatchingUseCaseImpl::new().visit_regex_matches(regex, text, &mut |found| {
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
