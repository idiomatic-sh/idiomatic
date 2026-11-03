//! Graph preparation and row memo acquisition are scoped inside the same owner.
use super::*;
use crate::{
    application::file_dag::{cached_graph::CachedGraph, workspace::InvocationWorkspace},
    domain::{
        operating::types::OperatorContext,
        operator::contract::{
            graph_view::CachedGraphRow,
            resources::ResourceProgram,
            types::{EvaluationMode, OperatorContract},
        },
        table_model::types::Rows,
    },
};
use ryo_language::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::Input;
impl DerivedOwner {
    pub(in crate::application::file_dag) fn prepare_graph(
        &self,
        contract: &OperatorContract,
        admission: &InvocationWorkspace,
        mode: EvaluationMode,
        program: &ResourceProgram,
        logical: &[Input],
        rows: &[Rows<'_>],
    ) -> Result<Option<CachedGraph>, OperatingError> {
        CachedGraph::prepare(
            contract,
            admission,
            mode,
            program,
            logical,
            rows,
            self.backend.borrow_mut().as_mut(),
        )
    }
    pub(in crate::application::file_dag) fn graph_row<'a>(
        &self,
        graph: &'a mut CachedGraph,
        row: &[Value],
    ) -> Result<CachedGraphRow<'a>, OperatingError> {
        graph.row(row, self.backend.borrow_mut().as_mut())
    }
}
