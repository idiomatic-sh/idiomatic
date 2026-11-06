//! The types of the `hosted` concept: the relations a host declares to a run, and their rows. Plain data, public fields and derives, with no constructor and no inherent impl.

/// A relation the host declares, as the compiler is told of it: the name a query writes and the columns its rows have.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedRelation {
    /// The name a query writes: text that came from the host.
    pub name: HostedName,
    /// The columns, in order.
    pub columns: Vec<HostedColumn>,
}

/// The name of a relation the host declares: text that came from the host.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct HostedName(pub Box<str>);

/// A column of a relation the host declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostedColumn {
    /// The name a query writes for it: text that came from the host.
    pub name: ColumnLabel,
    /// What it holds.
    pub kind: ryo_relations::domain::relations::types::CellKind,
}

/// The name of a column of a relation the host declares: text that came from the host.
#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ColumnLabel(pub Box<str>);

/// A relation the host declares beside ryo's own: a name a rule writes, the columns its rows have, and the rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRelation {
    /// The name a rule writes, as the host chose it: text that came from the host.
    pub name: HostName,
    /// The columns, in order.
    pub columns: Vec<HostColumn>,
    /// The rows, each with a cell for every column.
    pub rows: Vec<HostRow>,
}

/// The name of a relation the host declares: text that came from the host.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostName(pub Box<str>);

/// A column of a relation the host declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostColumn {
    /// The name a rule writes for it: text that came from the host.
    pub name: HostColumnName,
    /// What it holds.
    pub kind: HostKind,
}

/// The name of a column of a relation the host declares: text that came from the host.
#[derive(Clone, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostColumnName(pub Box<str>);

/// What a column of a relation the host declares holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostKind {
    /// A file of the repository, by its path.
    File,
    /// A name or any text.
    Text,
    /// A whole number.
    Number,
}

/// One row of a relation the host declares: a cell for every column. A file is its path relative to the root, and a path the run does
/// not list is no file; a span is no value of a column the host declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostRow(pub Vec<crate::domain::records::types::Cell>);

/// Versioned typed bytes for the exact host declaration and row population.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostIdentity(pub Vec<u8>);

/// Text bytes written into an identity, length first.
#[derive(Clone, Copy, Debug)]
pub struct HostIdentityText<'a>(pub &'a [u8]);
