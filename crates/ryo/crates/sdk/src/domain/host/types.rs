//! The types of the `host` concept: a relation the host declares beside ryo's own, which a lint reads with its rows and a query is
//! validated against without them.

/// A relation the host declares, without its rows: what a query is checked against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostDeclaration {
    /// The name a query writes, as the host chose it.
    pub name: HostRelationName,
    /// The columns, in order.
    pub columns: Vec<HostColumn>,
}

/// The name of a relation the host declares.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostRelationName(pub Box<str>);

impl std::fmt::Display for HostRelationName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// A column of a relation the host declares.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostColumn {
    /// The name a rule writes for it.
    pub name: HostColumnName,
    /// What it holds.
    pub kind: HostKind,
}

/// The name of a column of a relation the host declares.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct HostColumnName(pub Box<str>);

impl std::fmt::Display for HostColumnName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

/// What a column of a relation the host declares holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum HostKind {
    /// A file of the repository, by its path.
    File,
    /// A name or any text.
    Text,
    /// A whole number.
    Number,
}

// The conversions into the engine's columns are the crate's own: a public `From` impl would put the engine's types on the SDK's
// surface.
impl HostColumn {
    /// The column as the engine reads a host relation's.
    pub(crate) fn into_engine(self) -> ryo_values::domain::hosted::types::HostColumn {
        ryo_values::domain::hosted::types::HostColumn {
            name: ryo_values::domain::hosted::types::HostColumnName(self.name.0),
            kind: self.kind.into_engine(),
        }
    }
}

impl HostKind {
    /// The kind as the engine reads a host column.
    pub(crate) const fn into_engine(self) -> ryo_values::domain::hosted::types::HostKind {
        use ryo_values::domain::hosted::types::HostKind as Engine;
        match self {
            Self::File => Engine::File,
            Self::Text => Engine::Text,
            Self::Number => Engine::Number,
        }
    }
}
