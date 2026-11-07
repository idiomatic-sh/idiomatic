//! The types of the `model` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use ryo::languages;

use crate::domain::syntax::types::Level;

/// One section: the heading that opens it, how deep it is, what holds it and where it ends.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Section {
    /// The heading that opens it.
    pub heading: languages::Position,
    /// The heading of the section that holds it, or the document when no heading does.
    pub parent: languages::Position,
    /// Where it ends: where the next heading of the same level or a shallower one starts, or the end of the file.
    pub end: languages::Offset,
    /// How deep its heading is.
    pub level: Level,
    /// The section that holds it, or the document.
    pub up: Up,
}

/// The sections of one file, in the order of their headings, which the worker keeps between files.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Sections(pub Vec<Section>);

/// What holds an element: a section, or the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Holding {
    /// A section holds it.
    Section(Section),
    /// No heading does: the document does.
    Document,
}

/// What opens the scope at an element.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Scoped {
    /// The document, which encloses itself.
    Document,
    /// A heading, which opens a section.
    Section(Section),
    /// Neither.
    Nothing,
}

/// The place of a section among the sections of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SectionPlace(pub usize);

/// What holds a section.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Up {
    /// The section at this place.
    Section(SectionPlace),
    /// The document.
    Document,
}
