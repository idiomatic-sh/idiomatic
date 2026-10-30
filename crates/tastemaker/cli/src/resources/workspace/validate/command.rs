//! `validate`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase as _;

use crate::outcome::{Ending, Outcome};

/// The arguments of `validate`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(after_help = "Examples:\n  tm validate\n  tm validate rule/<name>")]
pub struct ValidateArguments {
    /// Prove only the versions whose id holds one of these texts, `<package>/<kind>/<name>@<version>`.
    pub(super) only: Vec<String>,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer.
pub(super) fn answer(
    arguments: ValidateArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded + Ending + use<>, Outcome> {
    let request = super::schemas::request(arguments.only, edge);
    super::composition::build(
        &edge.store,
        &super::connections::connect(&edge.store, &edge.start, &edge.build, edge.writing),
    )
    .validate(request)
    .map(super::schemas::answer)
    .map_err(|error| crate::output::fail(&error, Outcome::Error))
}

/// Runs `validate`: proves each version whose digest moved, looks up the rest, and prints each failure; ends as a
/// failure when a version does not hold.
pub fn run(arguments: ValidateArguments, edge: &crate::edge::Edge) -> Outcome {
    let format = edge.format;
    match answer(arguments, edge) {
        Ok(answer) => crate::output::present(format, &answer, answer.ending()),
        Err(outcome) => outcome,
    }
}
