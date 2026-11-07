//! `down`: parses, checks, composes, calls the service, presents.

use crate::outcome::Outcome;

/// The arguments of `down`.
#[derive(Clone, Copy, Debug, clap::Args)]
#[command(after_help = "Examples:\n  asymptote down\n  asymptote down -n")]
pub struct DownArguments;

/// Checks the arguments, composes the products, calls each once and shapes their answer, with the outcome the command
/// ends as: an error when a product could not answer.
pub(super) fn answer(
    _arguments: DownArguments,
    edge: &crate::edge::Edge,
) -> Result<(impl crate::output::Encoded + use<>, Outcome), Outcome> {
    let request = super::schemas::request(edge);
    let workspace = super::composition::build(
        &request.start,
        &super::connections::connect(&edge.build, &edge.store),
    )
    .map_err(|error| {
        crate::output::fail(
            &asymptote_core::inbound::use_cases::down::errors::DownError::from(error),
            Outcome::Error,
        )
    })?;
    // At a workspace root it reaches what `check` reaches there: the root's own tools, then each member.
    let answered = workspace
        .products
        .iter()
        .map(|product| {
            (
                product.path.0.to_string(),
                product.service.down(
                    asymptote_core::inbound::use_cases::down::types::DownRequest {
                        start: product.root.clone(),
                        ..request.clone()
                    },
                ),
            )
        })
        .collect();
    super::schemas::answered(workspace.layout, answered)
}

/// Runs `down`: drains the lock of every finding the last check found fixed, in one lock operation.
pub fn run(arguments: DownArguments, edge: &crate::edge::Edge) -> Outcome {
    match answer(arguments, edge) {
        // A dry run's answer is its plan, which goes where the `--dry-run` switch says.
        Ok(answered) => {
            crate::output::dry_run::present(edge.writing, edge.format, &answered.0, answered.1)
        }
        Err(outcome) => outcome,
    }
}
