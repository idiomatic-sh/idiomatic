//! The types of the `pulldown_cmark` adapter: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::syntax::types::{Kind, Level};
use ryo::languages::SourcePath;
use ryo::languages::{ByteSlice, Count, Lossless, Offset};

/// Whether the parser's text is the code of a code block, the indentation of a block of HTML, or text of the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Within {
    /// The lines of a code block.
    Code,
    /// The indentation before the first line of a block of HTML, which the parser gives as text of no length; the white space
    /// between the tokens is written for it.
    Indent,
    /// Anything else.
    Prose,
}

/// The bytes an event of the parser spans.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Stretch {
    /// Where it starts.
    pub start: Count,
    /// Where it ends.
    pub end: Count,
}

/// What the adapter keeps while it writes the parser's events into the table as nodes and tokens.
pub(super) struct Writer<'b, 'w> {
    /// The tree, and where its last token ends.
    pub tree: Lossless<'b, 'w>,
    /// What the gap tokenizer keeps between two events.
    pub gap: Gap<'b>,
    /// The constructs open below the document.
    pub open: Vec<Kind>,
    /// Whether the text the parser gives is code.
    pub within: Within,
    /// The file's own path, which the destination of a link is resolved against.
    pub location: SourcePath<'b>,
    /// The sections open, the outermost first.
    pub sections: Vec<Opened>,
}

/// A section the adapter has open: how deep its heading is, and how many constructs were open around its heading.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Opened {
    /// How deep its heading is, from 1.
    pub level: Level,
    /// How many constructs were open when its heading started: the section is among their blocks.
    pub depth: Count,
}

/// Which of the open sections a heading, the end of a construct or the end of the file closes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Closing {
    /// A heading `level` deep among the blocks of `depth` constructs: the sections among deeper constructs, and those among the same
    /// blocks whose headings are as deep or deeper.
    Heading {
        /// How many constructs are open around the heading.
        depth: Count,
        /// How deep the heading is.
        level: Level,
    },
    /// The end of the construct that `depth` constructs were open in: the sections among its blocks and deeper.
    Within(Count),
    /// The end of the file: every section.
    Every,
}

/// What the tokenizer of the bytes no event spans keeps: the bytes, the definitions of link references and how many it has passed, and
/// what the next marker is to carry.
pub(super) struct Gap<'b> {
    /// The bytes of the file.
    pub source: ByteSlice<'b>,
    /// The definitions of link references the parser found, in the order they start.
    pub definitions: Vec<Definition>,
    /// The first definition that has not been passed.
    pub passed: Passed,
    /// What the next marker of the construct just opened is to carry.
    pub pending: Pending,
}

/// The place of a definition among those of the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Passed(pub usize);

/// What the marker that opens a construct which names something carries: the path a link names, or the label of a footnote.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Carried(pub String);

/// Whether the next marker carries something.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Pending {
    /// It carries this.
    Carrying(Carried),
    /// It carries nothing.
    Nothing,
}

/// The definition of a link reference: the bytes it spans and the label it gives.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Definition {
    /// Where it starts.
    pub start: Offset,
    /// Where it ends.
    pub end: Offset,
    /// The label.
    pub label: Carried,
}

/// The part of a file's bytes that is UTF-8 text, which the library reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct MarkdownText<'a>(pub &'a str);
