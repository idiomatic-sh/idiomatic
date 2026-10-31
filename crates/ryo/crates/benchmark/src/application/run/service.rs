//! Preparation and decoding are deliberately outside the measured interval.
use crate::domain::diagnostics::errors::Error;
use crate::domain::measurements::types::Measurement;
use crate::domain::suite::types::RunRequest;
use crate::outbound::execution::interface::ExecutionPort;
use crate::outbound::linter::interface::LinterPort;

pub fn run(
    linter: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    request: &RunRequest,
) -> Result<Measurement, Error> {
    let invocation = linter.prepare(request)?;
    let output = execution.run(invocation)?;
    let wall = output.wall;
    let report = linter.decode(request, output)?;
    let mut expected = request.files.clone();
    expected.sort();
    if expected.windows(2).any(|pair| pair[0] == pair[1]) {
        return Err(Error::Invalid("duplicate input path".into()));
    }
    let mut actual = report.files.clone();
    actual.sort();
    if actual != expected {
        return Err(Error::Invalid(
            "tool input coverage differs from requested corpus".into(),
        ));
    }
    for finding in &report.diagnostics {
        if actual.binary_search(&finding.file).is_err() || finding.start > finding.end {
            return Err(Error::Invalid(format!("invalid diagnostic: {finding:?}")));
        }
    }
    Ok(Measurement { wall, report })
}
