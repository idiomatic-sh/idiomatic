//! The types of the `prose` concept: plain data, public fields and derives, with no constructor and no inherent impl.

/// What an inline element of a run adds to the text a reader reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Inline {
    /// Its own words.
    Words,
    /// A space: a break, or something a reader does not read as prose (code, math, raw HTML), which is masked.
    Mask,
    /// The words of what it holds: emphasis, a link, an image's description.
    Holds,
    /// Nothing: a marker, white space, or a block a run does not hold.
    Nothing,
}

/// Whether a run has a word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Wording {
    /// It has.
    Words,
    /// It has none.
    Silent,
}

/// Whether a space is owed before the next word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WordGap {
    /// A break or a mask came since the last word.
    Open,
    /// The last word came right before.
    Closed,
}

/// What has been written of a run so far.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Spoken {
    /// Whether a word has been written.
    pub any: Wording,
    /// Whether a space is owed.
    pub gap: WordGap,
}

/// Whether the code of a code span is read as words or masked.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Code {
    /// Masked by a space, as the text of a run masks it.
    Masked,
    /// Read as words, as a title's name spells it.
    Read,
}
