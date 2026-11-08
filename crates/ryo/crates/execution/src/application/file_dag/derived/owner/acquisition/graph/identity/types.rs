//! What the identity of an admitted population is proven over.
use ryo_execution_domain::domain::stored_graph::types::RelationPopulation;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// Whether native syntax-tree populations may prove an identity, or only stored ones.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::application::file_dag::derived::owner::acquisition::graph) enum PopulationScope {
    Stored,
    Native,
}

/// A proven population with the original columns its rows expose.
pub(in crate::application::file_dag::derived::owner::acquisition::graph) struct ProvenPopulation {
    pub population: RelationPopulation,
    pub columns: Vec<ColumnIndex>,
}
