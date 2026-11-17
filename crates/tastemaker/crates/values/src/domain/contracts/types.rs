//! The types of the `contracts` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use std::collections::BTreeSet;

use crate::domain::comments::types::ByteSpan;
use crate::domain::definitions::types::Exactness;
use crate::domain::paths::types::RepoPath;

/// A placed file, and the content its template renders for it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Contract {
    /// The placed file.
    pub target: RepoPath,
    /// Where the rendered content is given to the lint call, under `RENDERED`.
    pub rendered: RepoPath,
    /// The rendered content.
    pub text: RenderedContent,
    /// The template entry that placed the file, by its identity.
    pub identity: TemplateIdentity,
    /// Whether the file holds exactly the content.
    pub exact: Exactness,
}

/// One declaration a file makes: what kind it is, its name, the declarations that enclose it (outermost first), its
/// attributes (its decorations, its visibility and its type among them), and the paths of its decorations.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Declaration {
    /// Its kind: the refinement the language gives it, or the shared kind.
    pub kind: DeclarationKind,
    /// Its name.
    pub name: DeclarationName,
    /// The names of the declarations enclosing it, outermost first.
    pub owners: Vec<DeclarationName>,
    /// Its attributes, each as written without whitespace; a decoration one argument each.
    pub attributes: BTreeSet<AttributeText>,
    /// The paths its decorations name, when a file's declaration may carry no decoration its template's does not.
    pub decorations: Decorations,
}

/// Which decorations a file's declaration may carry beyond its template's.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Decorations {
    /// Any: what the file itself declares, its inner attributes, may say more than its template.
    #[default]
    Open,
    /// None whose path is not one of these: beside the decorations its template's content gives it, an extra one can
    /// disable what the template declares.
    Named(BTreeSet<DeclarationName>),
}

/// The shared relation a built-in rule of the comparison reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Syntax {
    /// What a file declares: `defines`.
    Defines,
    /// Its scopes: `scope`.
    Scopes,
    /// Its decorations: `decorates`.
    Decorates,
    /// The visibilities its declarations are written with: `visibility`.
    Visibility,
    /// The types its bindings are annotated with: `annotates`.
    Annotates,
    /// A data document's keys, with their values: `entry`.
    Entries,
    /// A data document's tables: `mapping`.
    Mappings,
    /// A data document's sequence elements: `sequence-element`.
    Elements,
    /// A document's runs of prose: `prose`.
    Prose,
    /// A file's literals, a document's code among them: `string-literal`.
    Literals,
}

/// One row of a shared relation the comparison reads, by the columns it compares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DataRow {
    /// Which relation it is of.
    pub syntax: Syntax,
    /// Its node.
    pub id: RowNode,
    /// The node that holds it.
    pub owner: RowNode,
    /// Its name: a declaration's, a decoration's path, a key, an element's place.
    pub key: RowKey,
    /// Its kind.
    pub kind: SyntaxKind,
    /// Its text: a declaration's refinement, a decoration's arguments, a value as written.
    pub text: WrittenValue,
    /// Where it is in its file.
    pub span: RowPlace,
}

/// The columns of one shared relation the comparison reads: the relation, the column its rows are keyed by, the node that
/// holds a row, its name, its kind and its text; an empty column is one the relation has not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Columns {
    /// The relation.
    pub relation: RelationText,
    /// The column a row is keyed by: its node.
    pub id: ColumnText,
    /// The node that holds it.
    pub owner: ColumnText,
    /// Its name.
    pub key: ColumnText,
    /// Its kind.
    pub kind: ColumnText,
    /// Its text.
    pub text: ColumnText,
}

/// A relation of the catalog, by its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RelationText(pub &'static str);

/// A column of a relation, by its name; empty for one the relation has not.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnText(pub &'static str);

/// The rows a file answered, one list for each shared relation the comparison reads.
#[derive(Clone, Debug, Default)]
pub struct Facts<'r> {
    /// `defines`.
    pub defines: Vec<&'r DataRow>,
    /// `scope`.
    pub scopes: Vec<&'r DataRow>,
    /// `decorates`.
    pub decorates: Vec<&'r DataRow>,
    /// `visibility`.
    pub visibility: Vec<&'r DataRow>,
    /// `annotates`.
    pub annotates: Vec<&'r DataRow>,
    /// `entry`.
    pub entries: Vec<&'r DataRow>,
    /// `mapping`.
    pub mappings: Vec<&'r DataRow>,
    /// `sequence-element`.
    pub elements: Vec<&'r DataRow>,
    /// `prose`.
    pub prose: Vec<&'r DataRow>,
    /// `string-literal`.
    pub literals: Vec<&'r DataRow>,
}

/// How a contract is compared, from what its rendered content answered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// By its data: it answered `entry` or `sequence-element`.
    Data,
    /// In order, block by block: an exact content that answered `prose`.
    Document,
    /// By its declarations: it answered `scope` or `defines`.
    Code,
    /// It answered nothing a comparison reads.
    Blind,
}

/// How a file differs from its contract.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Mismatch {
    /// A declaration the content makes that the file does not.
    Missing(Declaration),
    /// A declaration the file makes that an exact content does not.
    Extra(Declaration),
}

/// The content a template renders for a placed file.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedContent(pub Box<str>);

/// A template entry, by its identity: what a `#template` finding is named after.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct TemplateIdentity(pub Box<str>);

/// A kind of a row, as the relation writes it: a shared kind, a kind of value.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SyntaxKind(pub Box<str>);

/// A kind of declaration, as the comparison names it: a language's refinement, a block of a document, a key.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclarationKind(pub Box<str>);

/// The name of a declaration, or of one enclosing it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct DeclarationName(pub Box<str>);

/// An attribute of a declaration, as compared: written without whitespace.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AttributeText(pub Box<str>);

/// A node of a file, by the id its relation gives it.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowNode(pub Box<str>);

/// A row's name: a declaration's, a decoration's path, a key, the place of an element.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct RowKey(pub Box<str>);

/// A value of a document's data, as written.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct WrittenValue(pub Box<str>);

/// Where a row of a document's data is in its file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowPlace {
    /// At this byte range.
    At(ByteSpan),
    /// The relation does not say.
    Unplaced,
}

/// A file's text, as the comparison reads it.
#[derive(Clone, Copy, Debug)]
pub struct ComparedText<'a>(pub &'a str);

/// The text of one part of a file.
#[derive(Clone, Copy, Debug)]
pub struct PartText<'a>(pub &'a str);

/// What a comparison says, in a sentence: a mismatch, a block of a document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Said(pub String);

/// A query of the comparison's built-in rules, as text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComparisonQuery(pub String);

/// Whether one span or place holds another.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Holding {
    /// It does.
    Holds,
    /// It does not.
    Apart,
}

/// Whether a template's declaration is met by a file's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Meeting {
    /// It is: the same kind, name and owners, and every attribute of the template's.
    Met,
    /// It is not.
    Unmet,
}

impl std::fmt::Display for TemplateIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::fmt::Display for Said {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
