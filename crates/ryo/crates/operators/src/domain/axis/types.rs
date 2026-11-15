//! What an axis reads from the engine: the candidates of a walk and whether each is admitted.





/// What an axis lowers to: its direction, the start and edge columns, the domain's id column, what the domain admits (a
/// filter's comparisons) and the columns of an answer row.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AxisParts {
    /// Where the walk goes from a start.
    pub(crate) direction: ryo_values::domain::plan_parts::types::Direction,
    /// The start row's entity column.
    pub(crate) id: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The parent column of an edge row.
    pub(crate) parent: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The child column of an edge row.
    pub(crate) child: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The entity column of a domain row.
    pub(crate) domain_id: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The domain's comparisons, compiled as a filter's: a domain row is admitted when they hold.
    pub(crate) domain: crate::domain::select::types::SelectParts,
    /// The columns of an answer row; none means the start row then the entity.
    pub(crate) output: Vec<ryo_values::domain::plan_parts::types::JoinColumn>,
}

/// The axis operation's columns and direction, borrowed from it for its lowering.
#[derive(Clone, Copy, Debug)]
pub(crate) struct AxisShape<'a> {
    /// Where the walk goes from a start.
    pub(crate) direction: ryo_values::domain::plan_parts::types::Direction,
    /// The start row's entity column.
    pub(crate) id: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The parent column of an edge row.
    pub(crate) parent: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The child column of an edge row.
    pub(crate) child: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The entity column of a domain row.
    pub(crate) domain_id: ryo_values::domain::plan_parts::types::ColumnIndex,
    /// The columns of an answer row.
    pub(crate) output: &'a [ryo_values::domain::plan_parts::types::JoinColumn],
}
