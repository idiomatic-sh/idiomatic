//! `up`'s workload over the bench product, a dry run so every run records the same.

use asymptote_cli::resources::ratchet::up::command::{UpArguments, run};

/// A dry-run `up` over the bench product, run once per round, which must succeed.
///
/// # Errors
///
/// When the bench product cannot be made or the command line does not parse.
pub(crate) fn workload() -> Result<crate::measure::Measured, Box<dyn std::error::Error>> {
    let directory = crate::fixtures::product()?;
    let edge = crate::fixtures::dry(crate::fixtures::edge(&directory));
    crate::measure::rounds(
        "ratchet/up",
        || crate::fixtures::arguments::<UpArguments>(&["up", "--reason", "bench"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            if outcome != asymptote_cli::outcome::Outcome::Success {
                return Err("expected: `up` ends as it should".into());
            }
            Ok(())
        },
    )
}
