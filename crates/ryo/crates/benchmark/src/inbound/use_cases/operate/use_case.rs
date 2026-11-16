//! The `operate` use case: the struct holding only the ports it calls.
use super::constants::REFERENCE_NOT_VERIFIED;
use super::interface::OperateUseCase;
use super::types::{OperationReport, OperationRequest, OperationVerdict};
use crate::application::inventory_verification::service as inventory_verification;
use crate::application::registration::service::register;
use crate::domain::operations::constants::{COLLECTOR, INVENTORY, WORKER};
use crate::domain::operations::service::{document, layout, worker_arguments};
use crate::domain::operations::types::BenchOperation;
use crate::domain::suite::types::inventory::value::InventoryVerdict;
use crate::outbound::{
    executables::{
        interface::ExecutablesPort,
        types::{BenchPackage, RunArguments, RunVerdict, Sealed},
    },
    execution::interface::ExecutionPort,
    inventory::interface::InventoryPort,
    receipts::interface::ReceiptsPort,
    suite::{interface::SuitePort, types::ArtifactPath},
};
use ryo_benchmark_contracts::domain::documents::types::text::value::Sha256Text;
use ryo_benchmark_contracts::domain::measurements::constants::MEASURED_WORKERS;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::constants::VERIFY_TOOL;
use ryo_benchmark_contracts::domain::requests::types::{
    RegisteredCoverage, case_shard::value::ShardCount,
};
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
    /// Runs the verification's rules.
    scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
    /// The processors the bench may run on: a bench is measured on four.
    host: Arc<dyn crate::outbound::host::interface::HostPort>,
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
        scheduler: Arc<dyn crate::outbound::scheduler::interface::SchedulerPort>,
        host: Arc<dyn crate::outbound::host::interface::HostPort>,
    ) -> Self {
        Self {
            executables,
            suite,
            inventory,
            receipts,
            execution,
            scheduler,
            host,
        }
    }
}

impl OperateUseCase for OperateUseCaseImpl {
    fn operate(
        &self,
        request: &OperationRequest,
        output: &mut dyn std::io::Write,
    ) -> Result<OperationReport, Error> {
        // Every timed measure is taken on four processors; a run the bench tool's `taskset` did not pin is refused first.
        let processors = self.host.processors().0.get();
        if processors != MEASURED_WORKERS {
            return Err(Error::Invalid(format!(
                "a bench runs on {MEASURED_WORKERS} processors; this process may run on {processors}"
            )));
        }
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
        let _collector = self
            .executables
            .seal_current(&layout.root.join(COLLECTOR))?;
        // The reference tool's pinned bytes and version line, once for the run: the registration then pins its digest, and the
        // verification checks that digest before and after every rule.
        if self.executables.check_sealed(
            &worker.path,
            &RunArguments(vec![VERIFY_TOOL.into(), reference.path.clone().into()]),
        )? == RunVerdict::Failed
        {
            return Err(Error::Invalid(REFERENCE_NOT_VERIFIED.into()));
        }
        // The whole bench first: a rule directory that is neither a default nor a supporting rule, or a query that does not
        // read, is refused before any rule is registered.
        let _audit = crate::application::suite_auditing::service::audit(
            self.suite.as_ref(),
            &request.bench.0,
            &profile,
        )?;
        let registration = register(
            self.suite.as_ref(),
            self.inventory.as_ref(),
            &request.bench.0,
            &reference,
            &worker,
            &layout.root,
        )?;
        // Every default's cases once; a bench whose cases fail measures nothing.
        let verified = inventory_verification::verify(
            &self.inventory.load(&registration)?,
            &layout.root.join(INVENTORY),
            self.suite.as_ref(),
            self.receipts.as_ref(),
            self.execution.as_ref(),
            self.inventory.as_ref(),
            &RegisteredCoverage::Full,
            self.scheduler.as_ref(),
            ShardCount(1),
        )?;
        let answer = match verified {
            InventoryVerdict::Failed => OperationVerdict::Failed,
            InventoryVerdict::Passed => OperationVerdict::from(self.executables.run_sealed(
                &worker.path,
                &RunArguments(worker_arguments(
                    BenchOperation::Bench,
                    &layout,
                    &reference.path,
                )),
                &document(&layout),
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
