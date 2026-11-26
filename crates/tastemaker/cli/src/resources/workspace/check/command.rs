//! `check`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::check::interface::CheckUseCase as _;
use tastemaker_core::inbound::use_cases::validate::interface::ValidateUseCase;

use crate::outcome::{Ending, Outcome};
use crate::output::Format;

/// The arguments of `check`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling schema module and that module's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(
    after_help = "Examples:\n  tm check\n  tm check crates/core --format sarif\n  tm check --fix"
)]
pub struct CheckArguments {
    /// Report only the findings at or beneath these paths, relative to the current directory.
    pub(super) paths: Vec<std::path::PathBuf>,
    /// Rewrite a stale or missing `tastemaker.lock`, then check.
    #[arg(long)]
    pub(super) fix: bool,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer for `format`, with the outcome the
/// answer ends the command as; when it cannot answer, the error is said on stderr and its outcome returned. The composed services
/// come back beside the answer, for `run` to keep until the process ends.
fn answered(
    arguments: CheckArguments,
    edge: &crate::edge::Edge,
    format: Format,
) -> Result<(impl crate::output::Encoded + Ending + use<>, impl Sized + use<>), Outcome> {
    let request = super::schemas::request(arguments.paths, arguments.fix, edge)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))?;
    let (check, validate) = super::composition::build(
        &edge.store,
        &super::connections::connect(&edge.store, &edge.start, &edge.build, edge.writing),
    );
    proved(&validate, edge)?;
    let answer = check
        .check(request)
        .map(|report| super::schemas::answer(&report, format))
        .map_err(|error| crate::output::fail(&error, Outcome::Error))?;
    Ok((answer, (check, validate)))
}

/// The answer of `answered`, without the services.
#[cfg(test)]
pub(super) fn answer(
    arguments: CheckArguments,
    edge: &crate::edge::Edge,
    format: Format,
) -> Result<impl crate::output::Encoded + Ending + use<>, Outcome> {
    answered(arguments, edge, format).map(|(answer, _composed)| answer)
}

/// Proves every definition as `tm validate` does, a lookup for each whose proof is current; when one does not hold, the
/// check cannot run: it says which and why on stderr, in every format, so the output a format promises (a SARIF log) is
/// never another text, and ends as an error.
fn proved(validate: &impl ValidateUseCase, edge: &crate::edge::Edge) -> Result<(), Outcome> {
    let report = validate
        .validate(super::schemas::proving(edge))
        .map_err(|error| crate::output::fail(&error, Outcome::Error))?;
    // Each definition and problem is said as an error of its own, and the check ends as an error; none, and it goes on.
    super::schemas::unproven(&report)
        .iter()
        .fold(None, |_, error| {
            Some(crate::output::fail(error, Outcome::Error))
        })
        .map_or(Ok(()), Err)
}

/// Runs `check`: places the workspace, lints it, and prints the findings in the format `--format` chose;
/// ends as a failure when there is a finding, every finding being an error.
pub fn run(arguments: CheckArguments, edge: &crate::edge::Edge) -> Outcome {
    let format = edge.format;
    match answered(arguments, edge, format) {
        Ok((answer, composed)) => {
            let outcome = crate::output::present(format, &answer, answer.ending());
            // The command's last act: the answer is printed and the stores are written, so the process ends without freeing the engine
            // the check ran on (the plans, tables and strings of a project, which take seconds to free): a binary's choice, as a
            // library user's linter drops normally.
            std::mem::forget(composed);
            outcome
        }
        Err(outcome) => outcome,
    }
}
