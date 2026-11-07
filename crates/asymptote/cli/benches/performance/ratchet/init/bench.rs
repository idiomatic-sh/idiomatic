//! `init`'s workload over a fresh git repository, a dry run so every run plans the same.

use asymptote_cli::resources::ratchet::init::command::{InitArguments, run};

/// A dry-run `init` over an empty git repository, run once per round, which must succeed.
///
/// # Errors
///
/// When the repository cannot be made or the command line does not parse.
pub(crate) fn workload() -> Result<crate::measure::Measured, Box<dyn std::error::Error>> {
    let directory = crate::fixtures::repository()?;
    let edge = crate::fixtures::dry(crate::fixtures::edge(&directory));
    crate::measure::rounds(
        "ratchet/init",
        || crate::fixtures::arguments::<InitArguments>(&["init"]),
        |arguments| {
            let outcome = run(arguments, &edge);
            if outcome != asymptote_cli::outcome::Outcome::Success {
                return Err("expected: `init` ends as it should".into());
            }
            Ok(())
        },
    )
}
