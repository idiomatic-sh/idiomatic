//! The types of the `explanations` concept: what `explain` says, as sections of Markdown lines; plain data with no constructor and
//! no inherent impl.

/// The heading of a section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionTitle(pub Box<str>);

/// One line of a section, as Markdown.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SectionLine(pub Box<str>);

/// One section of an explanation: a heading and its lines.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// The heading.
    pub title: SectionTitle,
    /// The lines, as Markdown.
    pub lines: Vec<SectionLine>,
}

/// The sections, written as one Markdown page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Page(pub Box<str>);

/// How many of something there are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Tally(pub usize);

/// What the workspace holds: the files it governs, the files placed and the rules in the lint call.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Totals {
    /// The files governed.
    pub governed: Tally,
    /// The files placed.
    pub placed: Tally,
    /// The rules in the lint call.
    pub rules: Tally,
}
