//! `query run`: parses, checks, composes, calls the service, presents.

#[cfg(test)]
use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
#[cfg(test)]
use ryo_core::inbound::use_cases::lint::types::LintRequest;

use crate::outcome::Outcome;
#[cfg(test)]
use ryo_core::inbound::use_cases::lint::errors::LintError;

/// The arguments of `query run`.
#[derive(Clone, Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to its sibling schemas file, whose tests read them; `pub` would publish them through the library's public arguments type, and private fields would hide them from those tests"
)]
#[command(
    after_help = "Examples:\n  ryo query run --text 'let f = select * from \"function\"; emit f subject file key(span);'\n  ryo query run \\\n    --text 'let f = select * from \"function\"; emit f subject file key(span);' \\\n    --text 'let d = select * from \"package-dependency\"; emit d subject source key(source);'"
)]
pub struct RunArguments {
    /// The RyoQL text to run. Give it more than once to run several queries over one read of the repository: the answer then starts
    /// with a `rule` column, the query's place among them from 1, and has every query's columns.
    #[arg(long, required = true)]
    pub(super) text: Vec<String>,
    /// The most memory the run may hold, in bytes; zero means what the machine has. The repository's files must fit in it, and the
    /// run stops when the process holds more while it reads them.
    #[arg(long, default_value_t = 0)]
    pub(super) memory_budget: u64,
    /// How many rows are shown; every row is counted.
    #[arg(long, default_value_t = ryo_core::inbound::use_cases::lint::constants::DEFAULT_LIMIT)]
    pub(super) limit: u64,
}

/// The old engine's answer, which the comparison test holds the executor's against until the old engine is deleted.
#[cfg(test)]
pub(super) fn answer(
    request: LintRequest,
    budget: ryo_values::domain::runs::types::Budget,
    edge: &crate::edge::Edge,
    out: Box<dyn std::io::Write + Send>,
) -> Result<(), LintError> {
    let store = super::schemas::store(edge, &request.session.root);
    super::composition::build(
        out,
        edge,
        store,
        request.session.root.clone(),
        budget,
        ryo_values::domain::runs::types::WorkersAsked(0),
    )
    .lint(request)
    .map(|_| ())
}

/// Answers through the SDK's `query run` and writes the answer to `out` as the edge's format prints it: one whole document for JSON,
/// written when the run ends well, row by row for the others.
pub(super) fn answered(
    texts: &[String],
    limit: u64,
    memory_budget: u64,
    edge: &crate::edge::Edge,
    out: Box<dyn std::io::Write + Send>,
) -> Result<(), super::schemas::RunSchemaError> {
    crate::output::written(
        edge.format,
        out,
        |rows| {
            let request = super::schemas::request(edge.start.0.to_string(), texts, limit)?;
            let workers = std::num::NonZeroUsize::new(
                ryo_values::domain::runs::service::workers_to_start(
                    ryo_values::domain::runs::types::WorkersAsked(0),
                    edge.machine,
                )
                .0 as usize,
            )
            .unwrap_or(std::num::NonZeroUsize::MIN);
            let report = ryo_sdk::query::run(
                crate::languages::linked_sdk(),
                &ryo_sdk::linter::WorkerPool::from(ryo_sdk::linter::Workers(workers)),
                super::schemas::memory(memory_budget),
                request,
            )
            .map_err(super::schemas::RunSchemaError::Failed)?;
            super::rendering::write(&report, &crate::output::schemas::rows(rows, edge.format))
        },
        super::schemas::unwritten,
    )
}

/// Runs `query run`: checks its arguments through its schemas, connects and composes its
/// service, calls it once, and prints the answer in the format `--format` chose: row by row as
/// the rows are produced, except as JSON, which is one whole document written when the run ends well.
pub fn run(arguments: RunArguments, edge: &crate::edge::Edge) -> Outcome {
    let format = edge.format;
    if let Err(error) = super::schemas::request(edge.start.0.to_string(), &arguments.text, arguments.limit) {
        return crate::output::fail(&error, Outcome::Failure);
    }
    let mut ended = Ok(());
    let outcome = crate::output::stream(format, Outcome::Success, |out| {
        ended = answered(&arguments.text, arguments.limit, arguments.memory_budget, edge, out);
    });
    match super::schemas::delivered(ended) {
        Ok(()) => outcome,
        Err(error) => crate::output::fail(&error, Outcome::Error),
    }
}
