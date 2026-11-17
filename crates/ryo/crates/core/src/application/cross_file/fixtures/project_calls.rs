//! An operators port that counts the nodes a project phase runs after the files and the rows they answer, and otherwise runs
//! as the operators do.

#[derive(Debug, Default)]
pub(crate) struct ProjectCalls {
    /// How many nodes ran after the files.
    pub(crate) runs: std::sync::atomic::AtomicUsize,
    /// How many rows the nodes that ran after the files answered.
    pub(crate) rows: std::sync::atomic::AtomicUsize,
}

impl ryo_execution_runtime::outbound::operators::interface::OperatorsPort for ProjectCalls {
    fn plan(
        &self,
        program: &ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram<'_>,
    ) -> Result<
        ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        ryo_operators::inbound::use_cases::run::fixtures::planned(program)
    }
    fn run(
        &self,
        plan: &ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan,
        node: ryo_operators::domain::physical_plan::types::PlanNodeId,
        buffers: &mut ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers,
        inputs: &dyn ryo_operators::domain::physical_plan::types::PlanInputs,
        resources: &mut dyn ryo_operators::domain::physical_plan::types::Resources,
    ) -> Result<(), ryo_operators::domain::contract::types::PortableOperatorFault> {
        use ryo_operators::domain::physical_plan::types::NodeRows;
        use ryo_operators::domain::physical_plan::types::Resources as _;
        use std::sync::atomic::Ordering;
        // After the files no file is open: a node run then reads no source.
        let after_the_files = resources.source().0.is_empty();
        ryo_operators::inbound::use_cases::run::fixtures::ran(
            plan, node, buffers, inputs, resources,
        )?;
        if after_the_files {
            let _runs = self.runs.fetch_add(1, Ordering::SeqCst);
            if let NodeRows::Materialized(table) = buffers.rows(node) {
                let _rows = self.rows.fetch_add(table.count.0, Ordering::SeqCst);
            }
        }
        Ok(())
    }
}
