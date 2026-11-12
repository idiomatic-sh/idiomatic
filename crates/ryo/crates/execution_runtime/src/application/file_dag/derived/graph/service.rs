//! The `graph` application operation.
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::graph_view::types::CachedGraphRow;
use ryo_execution_domain::domain::operator::contract::resources::types::ResourceProgram;
use ryo_execution_domain::domain::operator::contract::types::OperatorContract;
use ryo_execution_domain::domain::table_model::types::Rows;
use ryo_execution_store::application::file_dag::derived::owner::cached_graph::service::CachedGraph;
use ryo_execution_store::application::file_dag::derived::owner::service::DerivedOwner;
use ryo_execution_store::application::file_dag::workspace::service::InvocationWorkspace;
use ryo_relations::domain::relations::types::Value;

/// The graph cache the file's derived owner prepared for one invocation's axis or reach: its rows are read only
/// through that owner.
pub(crate) struct LeasedGraph(CachedGraph);

impl LeasedGraph {
    /// The file's cached graph for the contract's axis or reach, when the owner holds one for it.
    pub(crate) fn prepared(
        contract: &OperatorContract,
        admission: &InvocationWorkspace,
        program: &ResourceProgram,
        rows: &[Rows<'_>],
    ) -> Result<Option<Self>, OperatingError> {
        Ok(admission
            .owner()
            .derived()
            .prepare_graph(contract, admission, program, rows)?
            .map(Self))
    }
    /// The graph's answer for one input row, read through the file's derived owner.
    pub(crate) fn row<'a>(
        &'a mut self,
        owner: &DerivedOwner,
        row: &[Value],
    ) -> Result<CachedGraphRow<'a>, OperatingError> {
        owner.graph_row(&mut self.0, row)
    }
}
