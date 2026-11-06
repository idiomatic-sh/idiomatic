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
pub struct BoundObservationInput {
    /// The local query source name.
    pub name: ObservationBindingName,
    /// Required columns in order.
    pub columns: Vec<ObservationColumn>,
    /// The actual producer phase established by request planning.
    pub scope: ObservationScope,
}

impl From<ObservationKind> for ryo_relations::domain::relations::types::CellKind {
    /// The engine's cell kind of a semantic kind the SDK declares.
    fn from(kind: ObservationKind) -> Self {
        match kind {
            ObservationKind::Node => Self::Node,
            ObservationKind::Symbol => Self::Symbol,
            ObservationKind::File => Self::File,
            ObservationKind::Span => Self::Span,
            ObservationKind::Number => Self::Number,
            ObservationKind::Flag => Self::Flag,
        }
    }
}

impl From<ObservationColumn> for ryo_values::domain::observations::types::ObservationColumn {
    /// Preserve every declared semantic kind at the SDK boundary.
    fn from(column: ObservationColumn) -> Self {
        Self {
            name: ryo_values::domain::observations::types::ObservationColumnName(column.name.0),
            kind: column.kind.into(),
        }
    }
}

impl From<ObservationScope> for ryo_values::domain::plans::types::StepScope {
    /// The planner's step scope of the phase a declared input is established in.
    fn from(scope: ObservationScope) -> Self {
        match scope {
            ObservationScope::PerFile => Self::PerFile,
            ObservationScope::CrossFile => Self::CrossFile,
        }
    }
}

impl From<BoundObservationInput>
    for ryo_values::domain::observations::types::BoundObservationInput
{
    /// Carry the same explicit context through validation and compilation.
    fn from(input: BoundObservationInput) -> Self {
        Self {
            name: ryo_values::domain::observations::types::ObservationBindingName(input.name.0),
            columns: input.columns.into_iter().map(Into::into).collect(),
            scope: input.scope.into(),
        }
    }
}
