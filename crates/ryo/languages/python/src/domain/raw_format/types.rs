//! Independent retained raw-token parse results and source occurrences.







/// Whether every original segment was observed completely.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RawCompleteness {
    Complete,
}
/// A preserved logical source token, including its envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawFormatToken<'a>(pub &'a str);
/// The exact borrowed body inside a validated original token envelope.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct RawFormatBody<'a>(pub &'a str);
impl From<RawCompleteness> for bool {
    fn from(value: RawCompleteness) -> Self {
        value == RawCompleteness::Complete
    }
}
