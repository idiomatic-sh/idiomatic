//! The names of the `contracts` concept: the shared relations the comparison reads, their columns, and where rendered
//! contents are read from. No word here is a language's: each is a relation of the catalog, a column of one, or a word of
//! one of its closed sets.

use super::types::{ColumnText, Columns, RelationText};

/// The directory, at the repository's root, the rendered contents are given to the lint call under: no file of the
/// repository is there, and no rule but the comparison's reads it.
pub const RENDERED: &str = ".tastemaker-rendered";

/// `defines`: what a file declares, by its shared kind, its refinement and its name.
pub const DEFINES: Columns = Columns {
    relation: RelationText("defines"),
    id: ColumnText("id"),
    owner: ColumnText("owner"),
    key: ColumnText("name"),
    kind: ColumnText("kind"),
    text: ColumnText("refinement"),
};

/// `scope`: each scope of a file, under the scope that encloses it.
pub const SCOPE: Columns = Columns {
    relation: RelationText("scope"),
    id: ColumnText("child"),
    owner: ColumnText("parent"),
    key: ColumnText(""),
    kind: ColumnText(""),
    text: ColumnText(""),
};

/// `decorates`: a decoration of a declaration, by the path it names and the arguments it passes.
pub const DECORATES: Columns = Columns {
    relation: RelationText("decorates"),
    id: ColumnText("occurrence"),
    owner: ColumnText("declaration"),
    key: ColumnText("target"),
    kind: ColumnText(""),
    text: ColumnText("arguments"),
};

/// `visibility`: the visibility a declaration is written with.
pub const VISIBILITY: Columns = Columns {
    relation: RelationText("visibility"),
    id: ColumnText("occurrence"),
    owner: ColumnText("declaration"),
    key: ColumnText(""),
    kind: ColumnText(""),
    text: ColumnText("text"),
};

/// `annotates`: the type a binding is annotated with.
pub const ANNOTATES: Columns = Columns {
    relation: RelationText("annotates"),
    id: ColumnText("occurrence"),
    owner: ColumnText("owner"),
    key: ColumnText(""),
    kind: ColumnText(""),
    text: ColumnText("annotation"),
};

/// `entry`: a key of a data document, with its value.
pub const ENTRY: Columns = Columns {
    relation: RelationText("entry"),
    id: ColumnText("occurrence"),
    owner: ColumnText("owner"),
    key: ColumnText("key"),
    kind: ColumnText("kind"),
    text: ColumnText("text"),
};

/// `mapping`: a table of a data document.
pub const MAPPING: Columns = Columns {
    relation: RelationText("mapping"),
    id: ColumnText("id"),
    owner: ColumnText("owner"),
    key: ColumnText("ordinal"),
    kind: ColumnText("kind"),
    text: ColumnText(""),
};

/// `sequence-element`: an element of a data document's sequence, by its place.
pub const SEQUENCE_ELEMENT: Columns = Columns {
    relation: RelationText("sequence-element"),
    id: ColumnText("occurrence"),
    owner: ColumnText("owner"),
    key: ColumnText("ordinal"),
    kind: ColumnText("kind"),
    text: ColumnText("text"),
};

/// `prose`: a run of the text a reader of a document reads.
pub const PROSE: Columns = Columns {
    relation: RelationText("prose"),
    id: ColumnText("id"),
    owner: ColumnText("owner"),
    key: ColumnText(""),
    kind: ColumnText(""),
    text: ColumnText("text"),
};

/// `string-literal`: a literal, by its value; in a document, its code.
pub const STRING_LITERAL: Columns = Columns {
    relation: RelationText("string-literal"),
    id: ColumnText("occurrence"),
    owner: ColumnText("owner"),
    key: ColumnText(""),
    kind: ColumnText(""),
    text: ColumnText("value"),
};

/// The shared kinds whose declarations hold a body: what is declared within one is its body's, and no template compares
/// it.
pub const BODIES: [&str; 2] = ["function", "method"];

/// The shared kinds a template's content does not declare: what a file imports, which its `imports` hold, and the parts
/// of a declaration (its fields, its members, its type parameters and its parameters), which belong to its body.
pub const PARTS: [&str; 5] = [
    "import",
    "property",
    "enum-member",
    "type-parameter",
    "parameter",
];

/// The shared kinds of a declaration held to its template's decorations: a module, which a decoration the file adds can
/// leave out of every build.
pub const HELD: [&str; 1] = ["namespace"];

/// The shared kinds of a declaration whose annotated type is part of what it declares: a value's type, not a callable's.
pub const TYPED: [&str; 2] = ["constant", "variable"];

/// The shared kind of a titled part of a document, which nests the blocks after it.
pub const SECTION: &str = "section";

/// The kinds of a data value that is a scalar: its written value is part of its declaration.
pub const SCALARS: [&str; 5] = ["string", "integer", "float", "boolean", "datetime"];

/// The kinds of a data value that is a table: an element of a sequence that is one is judged whole.
pub const TABLES: [&str; 2] = ["mapping", "inline-mapping"];

/// The kind a data document's key or element is declared as.
pub const KEY: &str = "key";

/// What joins the segments of a key's path.
pub const KEY_PATH_SEPARATOR: &str = ".";

/// The name a document's run of prose is declared as, a block among its blocks.
pub const PROSE_BLOCK: &str = "prose";

/// The name a document's literal block is declared as, a block among its blocks.
pub const LITERAL_BLOCK: &str = "literal";

/// The attribute of a block written on one line.
pub const ONE_LINE: &str = "one-line";

/// The article a block is named with.
pub const ARTICLE: &str = "a";

/// What joins the words of a name.
pub const SPACE: &str = " ";

/// What joins a declaration's name to the names of those that enclose it, in a message.
pub const WITHIN: &str = " in ";

/// The brackets a decoration's argument list may nest, each opening with its closing.
pub const BRACKETS: [(char, char); 3] = [('(', ')'), ('[', ']'), ('{', '}')];

/// What separates the arguments of a decoration.
pub const ARGUMENT_SEPARATOR: char = ',';
