//! `query run`: parses, checks, composes, calls the service, presents.

use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use ryo_core::inbound::use_cases::lint::types::LintRequest;

use crate::outcome::CommandOutcome;
use ryo_core::inbound::use_cases::lint::errors::LintError;

/// The arguments of `query run`.
#[derive(Clone, Debug, clap::Args)]
#[expect(
    clippy::field_scoped_visibility_modifiers,
    reason = "clap fills these fields, the command hands them to the sibling `schemas.rs` and that file's tests read them; `pub` would publish them through the library's public arguments type, and private fields would hide them from those tests"
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

/// Composes the service over the stream, calls it once and writes the answer as it is produced, row by row.
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

/// Answers in the way the edge's format needs: one whole document for JSON, written when the run ends well, row by row for the
/// others.
pub(super) fn answered(
    request: LintRequest,
    budget: ryo_values::domain::runs::types::Budget,
    edge: &crate::edge::Edge,
    out: Box<dyn std::io::Write + Send>,
) -> Result<(), LintError> {
    crate::output::written(
        edge.format,
        out,
        |rows| answer(request, budget, edge, rows),
        super::schemas::unwritten,
    )
}

/// Runs `query run`: checks its arguments through its schemas, connects and composes its
/// service, calls it once, and prints the answer in the format `--format` chose: row by row as
/// the rows are produced, except as JSON, which is one whole document written when the run ends well.
pub fn run(arguments: RunArguments, edge: &crate::edge::Edge) -> CommandOutcome {
    let format = edge.format;
    let budget = super::schemas::budget(arguments.memory_budget);
    let request =
        match super::schemas::request(edge.start.0.to_string(), arguments.text, arguments.limit) {
            Ok(request) => request,
            Err(error) => return crate::output::fail(&error, CommandOutcome::Failure),
        };
    let rules = request.rules.clone();
    let mut ended = Ok(());
    let outcome = crate::output::stream(format, CommandOutcome::Success, |out| {
        ended = answered(request, budget, edge, out)
            .map_err(|error| super::schemas::failure(error, &rules));
    });
    match super::schemas::delivered(ended) {
        Ok(()) => outcome,
        Err(error) => crate::output::fail(&error, CommandOutcome::Error),
    }
}
