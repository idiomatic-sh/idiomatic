//! Positions within the request's selector list.

/// A position in the request's selector list.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct SelectorOffset(pub usize);

/// The selectors one rule uses: from `first` up to, not including, `end`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag) struct SelectorSpan {
    pub first: SelectorOffset,
    pub end: SelectorOffset,
}
