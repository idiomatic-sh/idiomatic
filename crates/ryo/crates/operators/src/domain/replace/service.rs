//! Literal replacement over a borrowed prepared matcher.
use crate::domain::contract::types::PortableOperatorFault;
use regex_automata::meta::Regex;
pub(crate) fn replace_compiled(
    regex: &Regex,
    text: &str,
    replacement: &str,
    output: &mut String,
) -> Result<(), PortableOperatorFault> {
    output.clear();
    let mut end = 0;
    for found in regex.find_iter(text) {
        output.push_str(
            text.get(end..found.start())
                .ok_or(PortableOperatorFault::ShapeMismatch)?,
        );
        output.push_str(replacement);
        end = found.end();
    }
    output.push_str(
        text.get(end..)
            .ok_or(PortableOperatorFault::ShapeMismatch)?,
    );
    Ok(())
}
