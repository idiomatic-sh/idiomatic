//! Correctness gates each measured sample, including cache and edited runs.
use crate::application::run::service::run;
use crate::domain::diagnostics::errors::Error;
use crate::domain::diagnostics::service::compare;
use crate::domain::measurements::types::{Pair, RunOrder};
use crate::domain::suite::service::require_complete;
use crate::domain::suite::types::RunRequest;
use crate::outbound::{execution::interface::ExecutionPort, linter::interface::LinterPort};

/// Execute both contestants in the specified order and retain only exact agreement.
pub fn measure(
    audit: &Audit,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
    order: RunOrder,
) -> Result<Pair, Error> {
    require_complete(audit)?;
    let (reference, candidate) = match order {
        RunOrder::ReferenceFirst => (
            run(reference, execution, request)?,
            run(candidate, execution, request)?,
        ),
        RunOrder::CandidateFirst => {
            let candidate = run(candidate, execution, request)?;
            (run(reference, execution, request)?, candidate)
        }
    };
    compare(&reference.report, &candidate.report)
        .map_err(|difference| Error::Invalid(format!("diagnostics differ: {difference:?}")))?;
    Ok(Pair {
        reference,
        candidate,
    })
}

use crate::domain::suite::types::audit::value::Audit;

/// Measure only the exact case-certified subset and immutable source bytes.
pub fn measure_certified(
    certificate: &crate::domain::suite::types::certificate::value::Certificate,
    suite: &dyn crate::outbound::suite::interface::SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
    order: RunOrder,
) -> Result<Pair, Error> {
    if suite.fingerprint(&certificate.paths)? != certificate.fingerprint {
        return Err(Error::Invalid(
            "certified profile, rules, cases or executable changed".into(),
        ));
    }
    if [
        reference.prepare(&certificate.probe)?,
        candidate.prepare(&certificate.probe)?,
    ] != certificate.profiles
    {
        return Err(Error::Invalid(
            "contestant command profile differs from case certification".into(),
        ));
    }
    for (tool, expected) in [
        (reference, &certificate.contestants[0]),
        (candidate, &certificate.contestants[1]),
    ] {
        if tool.prepare(request)?.executable.canonicalize()? != *expected {
            return Err(Error::Invalid(
                "contestant differs from case-certified executable".into(),
            ));
        }
    }
    let sources: Vec<_> = request
        .files
        .iter()
        .map(|file| request.root.join(file))
        .collect();
    let before = suite.fingerprint(&sources)?;
    let pair = measure(
        &certificate.coverage,
        reference,
        candidate,
        execution,
        request,
        order,
    )?;
    if suite.fingerprint(&certificate.paths)? != certificate.fingerprint {
        return Err(Error::Invalid("certified artifacts changed during paired observation".into()));
    }
    if suite.fingerprint(&sources)? != before {
        return Err(Error::Invalid(
            "corpus source changed during paired observation".into(),
        ));
    }
    Ok(pair)
}
