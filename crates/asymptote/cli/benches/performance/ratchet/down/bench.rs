//! `down`'s workload over the bench product.

use asymptote_cli::resources::ratchet::down::command::{DownArguments, run};

/// A dry-run `down` over the bench product, run once per round, which must succeed.
///
/// # Errors
///
/// When the bench product cannot be made or the command line does not parse.
pub(crate) fn workload() -> Result<crate::measure::Measured, Box<dyn std::error::Error>> {
    let directory = crate::fixtures::product()?;
    let edge = crate::fixtures::dry(crate::fixtures::edge(&directory));
    crate::measure::rounds(
        "ratchet/down",
        || crate::fixtures::arguments::<DownArguments>(&["down"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            if outcome != asymptote_cli::outcome::Outcome::Success {
                return Err("expected: `down` ends as it should".into());
            }
            Ok(())
        },
    )
}
