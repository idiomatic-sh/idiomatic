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
use crate::outbound::evidence::types::{MeasureClass, PublishedDocument};
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
    /// Keeps the measures of the operation's class in the worker's document.
    evidence: Arc<dyn crate::outbound::evidence::interface::EvidencePort>,
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
        evidence: Arc<dyn crate::outbound::evidence::interface::EvidencePort>,
    ) -> Self {
        Self {
            executables,
            suite,
            inventory,
            receipts,
            execution,
            scheduler,
            host,
            evidence,
        }
    }
}

impl OperateUseCase for OperateUseCaseImpl {
    fn operate(
        &self,
        request: &OperationRequest,
        output: &mut dyn std::io::Write,
        observations: &mut dyn std::io::Write,
    ) -> Result<OperationReport, Error> {
        self.operate_with(request, None, output, observations)
    }
}

impl OperateUseCaseImpl {
    /// Profile the registered projects with the same admission as a measurement.
    ///
    /// # Errors
    /// Returns a registration, tool, or correctness failure.
    pub fn profile(
        &self,
        request: &OperationRequest,
        other: Option<std::path::PathBuf>,
        output: &mut dyn std::io::Write,
        observations: &mut dyn std::io::Write,
    ) -> Result<OperationReport, Error> {
        self.operate_with(request, other, output, observations)
    }

    fn operate_with(
        &self,
        request: &OperationRequest,
        other: Option<std::path::PathBuf>,
        output: &mut dyn std::io::Write,
        observations: &mut dyn std::io::Write,
    ) -> Result<OperationReport, Error> {
        let profile_run = request.operation == BenchOperation::Profile;
        // Every timed measure is taken on four processors; a run the bench tool's `taskset` did not pin is refused first.
        let processors = self.host.processors().0.get();
        if !profile_run && processors != MEASURED_WORKERS {
            return Err(Error::Invalid(format!(
                "a bench runs on {MEASURED_WORKERS} processors; this process may run on {processors}"
            )));
        }
        let target = if profile_run {
            self.executables.target_for(&request.bench.0)?
        } else {
            self.executables.target()?
        };
        let mut layout = layout(&target, &request.bench, request.operation)?;
        if profile_run {
            let directory = &layout.root;
            std::fs::create_dir_all(directory)?;
            layout.root = tempfile::Builder::new()
                .prefix("run-")
                .tempdir_in(directory)?
                .keep();
        }
        let declared = self.suite.profile(ArtifactPath(&request.bench.0))?;
        if profile_run {
            let registered_corpus = request.bench.0.canonicalize()?;
            let registered_corpus = registered_corpus
                .parent()
                .and_then(std::path::Path::parent)
                .ok_or_else(|| Error::Invalid("benchmark has no registered corpus".into()))?;
            if self
                .suite
                .projects(ArtifactPath(registered_corpus))?
                .projects
                .is_empty()
            {
                return Err(Error::Invalid(
                    "benchmark has no registered workload to profile".into(),
                ));
            }
        }
        let tool = self.executables.reference(&declared.tool)?;
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
            observations,
        )? == RunVerdict::Failed
        {
            return Err(Error::Invalid(REFERENCE_NOT_VERIFIED.into()));
        }
        // The whole bench first: a rule directory that is neither a default nor a supporting rule, or a query that does not
        // read, is refused before any rule is registered.
        let _audit = crate::application::suite_auditing::service::audit(
            self.suite.as_ref(),
            &request.bench.0,
            &declared,
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
        if verified == InventoryVerdict::Failed {
            return Ok(OperationReport {
                verdict: OperationVerdict::Failed,
            });
        }
        let document = if profile_run {
            let heap = self.executables.seal_heap_worker(
                &BenchPackage(request.bench.0.clone()),
                &layout.root.join("heap").join(WORKER),
            )?;
            crate::application::corpus_profile::prepare(
                &layout.root,
                worker.path.clone(),
                heap.path,
                other,
            )?
        } else {
            document(&layout)
        };
        // Both operations own their finalized documents. A profile also saves the tool tables.
        let mut written = Vec::new();
        let ran = self.executables.run_sealed(
            &worker.path,
            &RunArguments(worker_arguments(
                request.operation,
                &layout,
                &reference.path,
            )),
            &document,
            &mut written,
            observations,
        )?;
        if ran == RunVerdict::Passed {
            if profile_run {
                writeln!(
                    observations,
                    "Profile tables: {}",
                    layout.root.join("profile.md").display()
                )?;
                writeln!(observations, "Profile images: {}", layout.root.display())?;
                output.write_all(&written)?;
            } else {
                let published = self.evidence.select(
                    &PublishedDocument(written),
                    MeasureClass::from(request.operation),
                )?;
                output.write_all(&published.0)?;
            }
            output.flush()?;
        }
        Ok(OperationReport {
            verdict: OperationVerdict::from(ran),
        })
    }
}

impl std::fmt::Debug for OperateUseCaseImpl {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(std::any::type_name::<Self>())
            .finish_non_exhaustive()
    }
}
