//! The core's workloads: a `check` of the fixture product over the fakes of its ports, warm (its tool's run reused, so
//! what is measured is what asymptote itself spends around the tools) and cold (the product and its use case built for
//! each round).

use asymptote_core::inbound::use_cases::check::fixtures::{request, scenario, use_case};
use asymptote_core::inbound::use_cases::check::interface::CheckUseCase as _;

/// The core's warm and cold `check`.
///
/// # Errors
///
/// The first error a check of the fixture or the CPU clock answers.
pub(crate) fn measured() -> Result<Vec<crate::measure::Measured>, Box<dyn std::error::Error>> {
    Ok(vec![warm()?, cold()?])
}

/// `check` over one fixture product and one use case, built once, so each round measures asymptote's own work around
/// a tool run it reuses.
fn warm() -> Result<crate::measure::Measured, Box<dyn std::error::Error>> {
    let product = scenario();
    let check = use_case(&product);
    let first = check.check(request())?;
    if first.outcome.new.len() != 2 {
        return Err(format!("expected: the fixture product's first check reports its two new violations; answered {}", first.outcome.new.len()).into());
    }
    crate::measure::rounds(
        "core/warm-check",
        || Ok(()),
        |()| {
            let report = check.check(request())?;
            if report.outcome.new.len() != 2 {
                return Err(format!("expected: a warm check reports the fixture product's two new violations; answered {}", report.outcome.new.len()).into());
            }
            Ok(())
        },
    )
}

/// `check` with the fixture product and its use case built in each round, so each round measures a first run.
fn cold() -> Result<crate::measure::Measured, Box<dyn std::error::Error>> {
    crate::measure::rounds_of(
        "core/cold-check",
        crate::measure::COLD_ROUNDS,
        || Ok(()),
        |()| {
            let product = scenario();
            let report = use_case(&product).check(request())?;
            if report.outcome.new.len() != 2 {
                return Err(format!("expected: a cold check reports the fixture product's two new violations; answered {}", report.outcome.new.len()).into());
            }
            Ok(())
        },
    )
}
