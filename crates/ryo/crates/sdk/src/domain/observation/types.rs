//! Public observation declarations; internal planner types never cross the SDK edge.

/// An output explicitly authored by a producer query.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationName(pub Box<str>);
/// A consumer query's local source name.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ObservationBindingName(pub Box<str>);
/// An ordered observation column's semantic name.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationColumnName(pub Box<str>);
/// One ordered column in a named input or output.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObservationColumn {
    /// The column spelling used by queries.
    pub name: ObservationColumnName,
    /// The actual semantic cell kind.
    pub kind: ObservationKind,
}
/// Semantic kinds accepted in an observation schema.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationKind {
    /// A syntax node.
    Node,
    /// Interned text.
    Symbol,
    /// A file identity.
    File,
    /// A source span.
    Span,
    /// A whole number.
    Number,
    /// A Boolean flag.
    Flag,
}
/// The evaluation phase of an explicitly declared input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ObservationScope {
    /// One source file's rows.
    PerFile,
    /// Rows published across the requested corpus.
    CrossFile,
}
/// The typed context required to validate a query consuming a named observation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BoundObservationSchema {
    /// The local query source name.
    pub name: ObservationBindingName,
    /// Required columns in order.
    pub columns: Vec<ObservationColumn>,
    /// The actual producer phase established by request planning.
    pub scope: ObservationScope,
}
