//! What the `matching` port asks of the outside: the occurrences of a prepared regex in a text.

use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_matching::inbound::use_cases::matching::types::CompiledRegex;
use ryo_matching::inbound::use_cases::matching::types::{GroupName, SearchText, TextSpan};

/// What every `matching` adapter implements.
pub trait MatchingPort: Send + Sync {
    /// Visits the occurrences of `regex` in `text` in source order and stops when `each` breaks.
    fn visit_regex_matches(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        each: &mut dyn FnMut(TextSpan) -> std::ops::ControlFlow<()>,
    ) -> std::ops::ControlFlow<()>;

    /// Visits each occurrence with the span of the named group, stopping when `each` breaks.
    ///
    /// # Errors
    ///
    /// `InvalidRegex` when the regex has no group of that name.
    fn visit_regex_captures(
        &self,
        regex: &CompiledRegex,
        text: SearchText<'_>,
        name: GroupName<'_>,
        each: &mut dyn FnMut(TextSpan, TextSpan) -> std::ops::ControlFlow<()>,
    ) -> Result<std::ops::ControlFlow<()>, MatchingError>;
}
