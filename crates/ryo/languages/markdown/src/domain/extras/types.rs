//! The types of the `extras` concept: plain data, public fields and derives, with no constructor and no inherent impl. An extra is a kind of the language's trees that mdast and micromark have no name for.

/// What a kind of tree of a Markdown syntax tree is that mdast and micromark lack. One closed list, each name once.
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    Hash,
    strum::EnumIter,
    strum::IntoStaticStr,
    strum::FromRepr,
    strum::EnumCount,
)]
#[strum(serialize_all = "kebab-case")]
#[repr(u16)]
pub(crate) enum Extra {
    /// The syntax between the events of the parser: the markers of blocks and spans, indentation, fences and the destinations of links. The standard's token types split it by the construct it belongs to; the parser gives none of that.
    Markup,
    /// Front matter between `---` lines at the start of the file, which mdast has through an extension, `mdast-util-frontmatter`.
    Yaml,
    /// Front matter between `+++` lines at the start of the file, which mdast has through an extension, `mdast-util-frontmatter`.
    Toml,
    /// Math on a block of its own, `$$a$$`, which mdast has through an extension, `mdast-util-math`.
    Math,
    /// Math in a line of text, `$a$`, which mdast has through an extension, `mdast-util-math`.
    InlineMath,
    /// The mark of a task done, `[x]`, which mdast has as the `checked` field of a list item.
    TaskChecked,
    /// The mark of a task to do, `[ ]`, which mdast has as the `checked` field of a list item.
    TaskUnchecked,
    /// A list of terms and their definitions: an extension the language leaves off, which the parser never gives.
    DefinitionList,
    /// A term of a definition list: an extension the language leaves off, which the parser never gives.
    DefinitionListTitle,
    /// A definition of a definition list: an extension the language leaves off, which the parser never gives.
    DefinitionListDefinition,
    /// Raised text: an extension the language leaves off, which the parser never gives.
    Superscript,
    /// Lowered text: an extension the language leaves off, which the parser never gives.
    Subscript,
    /// Bytes that are not text: a Markdown file is UTF-8.
    Error,
    /// A section: a heading and every block after it up to the next heading as deep or shallower among the same blocks, nested
    /// as the headings are. mdast has no node for it; the tree holds one so that a block's parent is its section.
    Section,
}
