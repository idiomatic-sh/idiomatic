//! The types of the `nodes` concept: plain data, public fields and derives, with no constructor and no inherent impl. A node is a type of mdast, named by the type in kebab case.

/// What a node of a Markdown syntax tree is. One closed list, each name once.
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
pub(crate) enum Node {
    /// The whole file: `root`.
    Root,
    /// A paragraph: `paragraph`.
    Paragraph,
    /// A heading, `# a` or a line underlined with `=` or `-`: `heading`. Its depth is data, which `level_of` reads.
    Heading,
    /// A block quote, `> a`, or one that is a note, a tip or a warning: `blockquote`.
    Blockquote,
    /// A block of code between fences or indented by four spaces: `code`.
    Code,
    /// Raw HTML, a block of it or in a line of text: `html`.
    Html,
    /// A list of items, marked or numbered: `list`.
    List,
    /// One item of a list: `list-item`.
    ListItem,
    /// A footnote's definition, `[^a]: text`: `footnote-definition`.
    FootnoteDefinition,
    /// A table: `table`.
    Table,
    /// A row of a table, the row of its column names among them: `table-row`.
    TableRow,
    /// A cell of a table: `table-cell`.
    TableCell,
    /// Emphasis, `*a*`: `emphasis`.
    Emphasis,
    /// Strong emphasis, `**a**`: `strong`.
    Strong,
    /// Struck text, `~~a~~`: `delete`.
    Delete,
    /// A link written with its destination, `[a](b)`, or `<b>`: `link`.
    Link,
    /// A link written with a label, `[a][b]`, `[a][]` or `[a]`: `link-reference`.
    LinkReference,
    /// An image written with its destination, `![a](b)`: `image`.
    Image,
    /// An image written with a label, `![a][b]`: `image-reference`.
    ImageReference,
    /// The definition of a link reference, `[a]: b`: `definition`.
    Definition,
    /// Text: `text`.
    Text,
    /// Code in a line of text, `` `a` ``: `inline-code`.
    InlineCode,
    /// A line break forced by two spaces or a backslash: `break`.
    Break,
    /// A thematic break, `---`: `thematic-break`.
    ThematicBreak,
    /// A reference to a footnote, `[^a]`: `footnote-reference`.
    FootnoteReference,
}
