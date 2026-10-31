//! Correctness gates each measured sample, including cache and edited runs.
use crate::application::run::service::run;
use crate::domain::diagnostics::errors::Error;
use crate::domain::measurements::types::Pair;
use crate::domain::suite::types::RunRequest;
use crate::inbound::use_cases::audit::service::require_complete;
use crate::inbound::use_cases::compare::service::compare;
use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};

pub fn measure(
    audit: &Audit,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
) -> Result<Pair, Error> {
    require_complete(audit)?;
    let reference = run(reference, execution, request)?;
    let candidate = run(candidate, execution, request)?;
    compare(&reference.report, &candidate.report)
        .map_err(|difference| Error::Invalid(format!("diagnostics differ: {difference:?}")))?;
    Ok(Pair {
        reference,
        candidate,
    })
}

use crate::domain::suite::types::Audit;
