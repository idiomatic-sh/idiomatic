//! What an axis reads from the engine: the candidates of a walk and whether each is admitted.

/// What the candidates of one axis walk are.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisCandidates {
    /// Entities reached over an index of edges; each row appends the entity.
    Entities(ryo_relations::domain::relations::types::Count),
    /// Elements reached over a tree; each row appends the element's node, or the declared output.
    Elements(ryo_relations::domain::relations::types::Count),
}

/// Whether a candidate of an axis walk is admitted: an entity of the domain, an element its domain
/// predicate holds for or its dense marks hold.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AxisAdmission {
    /// The candidate is admitted.
    In,
    /// The candidate is not admitted.
    Out,
}

/// The engine's borrowed axis resources and its walk over them: a tree of elements, or an index of edges
/// with its domain membership.
pub trait AxisDriver {
    /// Walks in `direction` from the start the row holds in column `id`, and keeps the candidates in walk
    /// order; an axis with no resources, or a start of another file, has none.
    ///
    /// # Errors
    ///
    /// The engine retained an error acquiring or walking the axis, or reading the start.
    fn walk(
        &mut self,
        direction: ryo_values::domain::plan_parts::types::Direction,
        row: &[ryo_relations::domain::relations::types::Value],
        id: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Result<AxisCandidates, crate::domain::contract::types::PortableOperatorFault>;
    /// Whether a candidate of the last walk is admitted.
    ///
    /// # Errors
    ///
    /// The engine retained an error evaluating the domain.
    fn admits(
        &mut self,
        at: crate::domain::contract::types::VertexPlace,
    ) -> Result<AxisAdmission, crate::domain::contract::types::PortableOperatorFault>;
    /// One candidate entity, or the node of one candidate element.
    fn candidate(
        &self,
        at: crate::domain::contract::types::VertexPlace,
    ) -> Option<ryo_relations::domain::relations::types::Value>;
    /// One right cell of a candidate element.
    fn element_cell(
        &self,
        at: crate::domain::contract::types::VertexPlace,
        column: ryo_values::domain::plan_parts::types::ColumnIndex,
    ) -> Option<ryo_relations::domain::relations::types::Value>;
}

/// An engine driver that lends its axis resources and walk.
pub trait AxisEvaluation: crate::domain::contract::types::RowDriver {
    /// The engine's axis resources and walk.
    fn axis(&mut self) -> &mut dyn AxisDriver;
}

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
