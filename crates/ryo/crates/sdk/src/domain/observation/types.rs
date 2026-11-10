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

// The conversions into the planner's types are the crate's own: a public `From` impl would put the planner's types on the SDK's surface.
impl ObservationColumn {
    /// Preserve every declared semantic kind at the SDK boundary.
    pub(crate) fn into_engine(self) -> ryo_values::domain::observations::types::ObservationColumn {
        ryo_values::domain::observations::types::ObservationColumn {
            name: ryo_values::domain::observations::types::ObservationColumnName(self.name.0),
            kind: self.kind.into(),
        }
    }
}

impl ObservationScope {
    /// The planner's step scope of the phase a declared input is established in.
    pub(crate) const fn into_engine(self) -> ryo_values::domain::plan_parts::types::StepScope {
        match self {
            Self::PerFile => ryo_values::domain::plan_parts::types::StepScope::PerFile,
            Self::CrossFile => ryo_values::domain::plan_parts::types::StepScope::CrossFile,
        }
    }
}

impl BoundObservationInput {
    /// Carry the same explicit context through validation and compilation.
    pub(crate) fn into_engine(self) -> ryo_values::domain::observations::types::BoundObservationInput {
        ryo_values::domain::observations::types::BoundObservationInput {
            name: ryo_values::domain::observations::types::ObservationBindingName(self.name.0),
            columns: self.columns.into_iter().map(ObservationColumn::into_engine).collect(),
            scope: self.scope.into_engine(),
        }
    }
}
