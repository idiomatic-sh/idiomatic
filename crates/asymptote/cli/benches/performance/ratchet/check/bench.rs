//! `check`'s workload over the bench product, its tool's run reused.

use asymptote_cli::resources::ratchet::check::command::{CheckArguments, run};

/// `check` over the bench product, run once per round with its tool's run reused, which must
/// report the product's new violation.
///
/// # Errors
///
/// When the bench product cannot be made or the command line does not parse.
pub(crate) fn workload() -> Result<crate::measure::Measured, Box<dyn std::error::Error>> {
    let directory = crate::fixtures::product()?;
    let edge = crate::fixtures::edge(&directory);
    crate::measure::rounds(
        "ratchet/check",
        || crate::fixtures::arguments::<CheckArguments>(&["check"]),
        |arguments| {
            let outcome = run(arguments, &edge, |_| {});
            if outcome != asymptote_cli::outcome::Outcome::Failure {
                return Err("expected: `check` reports the new violation".into());
            }
            Ok(())
        },
    )
}
