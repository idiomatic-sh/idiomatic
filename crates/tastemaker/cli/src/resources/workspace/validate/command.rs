//! `validate`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase as _;

use crate::outcome::{Ending, Outcome};

/// The arguments of `validate`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling schema module and that module's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(after_help = "Examples:\n  tm validate\n  tm validate rule/<name>")]
pub struct ValidateArguments {
    /// Prove only the versions whose id holds one of these texts, `<package>/<kind>/<name>@<version>`.
    pub(super) only: Vec<String>,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer; the composed service comes back beside the
/// answer, for `run` to keep until the process ends.
fn answered(
    arguments: ValidateArguments,
    edge: &crate::edge::Edge,
) -> Result<(impl crate::output::Encoded + Ending + use<>, impl Sized + use<>), Outcome> {
    let request = super::schemas::request(arguments.only, edge);
    let composed = super::composition::build(
        &edge.store,
        &super::connections::connect(&edge.store, &edge.start, &edge.build, edge.writing),
    );
    let answer = composed
        .validate(request)
        .map(super::schemas::answer)
        .map_err(|error| crate::output::fail(&error, Outcome::Error))?;
    Ok((answer, composed))
}

/// The answer of `answered`, without the service.
#[cfg(test)]
pub(super) fn answer(
    arguments: ValidateArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded + Ending + use<>, Outcome> {
    answered(arguments, edge).map(|(answer, _composed)| answer)
}

/// Runs `validate`: proves each version whose digest moved, looks up the rest, and prints each failure; ends as a
/// failure when a version does not hold.
pub fn run(arguments: ValidateArguments, edge: &crate::edge::Edge) -> Outcome {
    let format = edge.format;
    match answered(arguments, edge) {
        Ok((answer, composed)) => {
            let outcome = crate::output::present(format, &answer, answer.ending());
            // The answer is printed and the proofs written: the process ends without freeing the engine, as `check` does.
            std::mem::forget(composed);
            outcome
        }
        Err(outcome) => outcome,
    }
}
