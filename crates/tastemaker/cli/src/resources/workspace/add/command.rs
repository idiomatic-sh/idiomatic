//! `add`: parses, checks, composes, calls the service, presents.

use tastemaker_core::inbound::use_cases::add::interface::AddUseCase as _;

use crate::outcome::Outcome;

/// The arguments of `add`.
#[derive(Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments struct, and private fields would hide them from those tests"
)]
#[command(
    after_help = "Examples:\n  tm add <definition> --at <path>\n  tm add <package>/<definition> --at <path> -n"
)]
pub struct AddArguments {
    /// The definition: `name`, or `package/name`.
    pub(super) definition: String,
    /// Where it applies, relative to the current directory: a rule's or spec's directory, a template's file.
    #[arg(long)]
    pub(super) at: std::path::PathBuf,
    /// A value the entry passes, `name=value`; repeat for more.
    #[arg(long = "param", value_name = "NAME=VALUE")]
    pub(super) params: Vec<String>,
}

/// Checks the arguments, composes the service, calls it once and shapes its answer.
pub(super) fn answer(
    arguments: AddArguments,
    edge: &crate::edge::Edge,
) -> Result<impl crate::output::Encoded, Outcome> {
    let dry = edge.writing == tastemaker_core::domain::writing::types::Writing::DryRun;
    let request =
        super::schemas::request(arguments.definition, arguments.at, arguments.params, edge)
            .map_err(|error| crate::output::fail(&error, Outcome::Error))?;
    let at = request.at.clone();
    let report = super::composition::build(&edge.store)
        .add(request)
        .map_err(|error| crate::output::fail(&error, Outcome::Error))?;
    // The command `add` says to run next is run where `add` ran: a `--root` that is not here is named in it.
    let root = (edge.here != edge.start).then_some(&*edge.start.0);
    super::schemas::answer(&report, dry, &at, root)
        .map_err(|error| crate::output::fail(&error, Outcome::Failure))
}

/// Runs `add`: writes the entry, and says the `scaffold` that writes its files.
pub fn run(arguments: AddArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments, edge) {
        Ok(answer) => {
            crate::output::present_planned(edge.format, edge.writing, &answer, Outcome::Success)
        }
        Err(outcome) => outcome,
    }
}
