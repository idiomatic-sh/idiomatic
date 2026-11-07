//! Correctness gates each measured sample, including cache and edited runs.
use crate::application::measurement::service::run;
use crate::domain::measurements::types::pair::value::Pair;
use crate::domain::suite::service::require_complete;
use crate::domain::suite::types::audit::value::Audit;
use crate::outbound::{
    context::interface::ContextPort, evidence::interface::EvidencePort,
    execution::interface::ExecutionPort, linter::interface::LinterPort,
};
use ryo_benchmark_contracts::domain::measurements::types::RunOrder;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::requests::types::RunRequest;

/// Execute both contestants, requiring all diagnostic findings and valid candidate execution; report incomparable reference metadata.
pub(super) fn measure(
    audit: &Audit,
    context: &dyn ContextPort,
    evidence: &dyn EvidencePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
    order: RunOrder,
) -> Result<Pair, Error> {
    require_complete(audit)?;
    let (reference, candidate) = match order {
        RunOrder::ReferenceFirst => (
            run(context, reference, execution, request)?,
            run(context, candidate, execution, request)?,
        ),
        RunOrder::CandidateFirst => {
            let candidate = run(context, candidate, execution, request)?;
            (run(context, reference, execution, request)?, candidate)
        }
    };
    crate::application::certified_measurement::service::pair(evidence, reference, candidate)
}

/// Measure only the exact case-certified subset and immutable source bytes.
pub(super) fn measure_certified(
    certificate: &crate::domain::suite::types::certificate::value::Certificate,
    mode: &ryo_benchmark_contracts::domain::measurements::types::MeasurementMode,
    suite: &dyn crate::outbound::suite::interface::SuitePort,
    context: &dyn ContextPort,
    evidence: &dyn EvidencePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
    order: RunOrder,
) -> Result<Pair, Error> {
    let baseline = crate::application::certified_measurement::service::admit(
        certificate, mode, suite, execution, reference, candidate, request,
    )?;
    let pair = measure(
        &certificate.coverage,
        context,
        evidence,
        reference,
        candidate,
        execution,
        request,
        order,
    )?;
    crate::application::certified_measurement::service::conclude(
        certificate, mode, suite, execution, &baseline, pair,
    )
}
