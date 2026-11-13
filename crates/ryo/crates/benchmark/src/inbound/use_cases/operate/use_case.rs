//! The `operate` use case: the struct holding only the ports it calls.
use super::interface::OperateUseCase;
use super::types::{OperationReport, OperationRequest, OperationVerdict};
use crate::application::inventory_verification::service as inventory_verification;
use crate::application::registration::service::register;
use crate::domain::operations::constants::{COLLECTOR, INVENTORY, WORKER};
use crate::domain::operations::service::{layout, worker_arguments};
use crate::domain::operations::types::BenchOperation;
use crate::domain::suite::types::inventory::value::InventoryVerdict;
use crate::outbound::{
    executables::{
        interface::ExecutablesPort,
        types::{BenchPackage, RunArguments, Sealed},
    },
    execution::interface::ExecutionPort,
    inventory::interface::InventoryPort,
    receipts::interface::ReceiptsPort,
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RegisteredCoverage;
use std::sync::Arc;

/// `operate`, over only the ports it calls, each injected by the edge.
pub struct OperateUseCaseImpl {
    /// Builds, seals and runs the executables.
    executables: Arc<dyn ExecutablesPort>,
    /// The bench's profile and files.
    suite: Arc<dyn SuitePort>,
    /// The registration and the inventory's results.
    inventory: Arc<dyn InventoryPort>,
    /// The native case receipts of the correctness operation.
    receipts: Arc<dyn ReceiptsPort>,
    /// The bounded process launch of the correctness operation.
    execution: Arc<dyn ExecutionPort>,
}

impl OperateUseCaseImpl {
    /// `operate` over the ports it calls, which the edge injects.
    #[must_use]
    pub fn new(
        executables: Arc<dyn ExecutablesPort>,
        suite: Arc<dyn SuitePort>,
        inventory: Arc<dyn InventoryPort>,
        receipts: Arc<dyn ReceiptsPort>,
        execution: Arc<dyn ExecutionPort>,
    ) -> Self {
        Self {
            executables,
            suite,
            inventory,
            receipts,
            execution,
        }
    }
}

impl OperateUseCase for OperateUseCaseImpl {
    fn operate(
        &self,
        request: &OperationRequest,
        output: &mut dyn std::io::Write,
    ) -> Result<OperationReport, Error> {
        let layout = layout(
            &self.executables.target()?,
            &request.bench,
            request.operation,
        )?;
        let profile = self.suite.profile(ArtifactPath(&request.bench.0))?;
        let tool = self.executables.reference(&profile.tool)?;
        let reference = Sealed {
            sha256: Sha256Text(self.suite.digest(ArtifactPath(&tool))?.0),
            path: tool,
        };
        let worker = self.executables.seal_worker(
            &BenchPackage(request.bench.0.clone()),
            &layout.root.join(WORKER),
        )?;
        let answer = match request.operation {
            BenchOperation::Correctness => {
                let registration = register(
                    self.suite.as_ref(),
                    self.inventory.as_ref(),
                    &request.bench.0,
                    &reference,
                    &worker,
                    &layout.root,
                )?;
                match inventory_verification::verify(
                    &self.inventory.load(&registration)?,
                    &layout.root.join(INVENTORY),
                    self.suite.as_ref(),
                    self.receipts.as_ref(),
                    self.execution.as_ref(),
                    self.inventory.as_ref(),
                    &RegisteredCoverage::Full,
                )? {
                    InventoryVerdict::Passed => OperationVerdict::Passed,
                    InventoryVerdict::Failed => OperationVerdict::Failed,
                }
            }
            BenchOperation::Measure => {
                let _collector = self
                    .executables
                    .seal_current(&layout.root.join(COLLECTOR))?;
                let _registration = register(
                    self.suite.as_ref(),
                    self.inventory.as_ref(),
                    &request.bench.0,
                    &reference,
                    &worker,
                    &layout.root,
                )?;
                OperationVerdict::from(self.executables.run_sealed(
                    &worker.path,
                    &RunArguments(worker_arguments(
                        BenchOperation::Measure,
                        &layout,
                        &reference.path,
                    )),
                    output,
                )?)
            }
            BenchOperation::Parity => OperationVerdict::from(self.executables.run_sealed(
                &worker.path,
                &RunArguments(worker_arguments(
                    BenchOperation::Parity,
                    &layout,
                    &reference.path,
                )),
                output,
            )?),
        };
        Ok(OperationReport { verdict: answer })
    }
}

impl std::fmt::Debug for OperateUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
