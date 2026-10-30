//! The command tree: `<resource> <command>`, one module per resource, and what the
//! command line itself answers: help, the version, usage errors, `--no-color` and a panic.

pub mod operator;
pub mod query;
pub mod relation;

/// The command line: a resource, then a command.
#[derive(clap::Parser)]
#[command(name = "ryo", version, about)]
#[command(propagate_version = true, after_help = SUPPORT, arg = no_color())]
struct Cli {
    /// The project's directory, where the command reads: this directory only, no ancestor is searched [default: the current directory].
    #[arg(long, global = true)]
    root: Option<String>,
    /// `--timings`, which every command takes: where the command's wall time went.
    #[command(flatten)]
    timings: crate::timings::TimingsArgs,
    /// `--format` and its spellings: how the answer is printed.
    #[command(flatten)]
    output: crate::output::FormatSwitch,
    /// The resource the command line names, with its command.
    #[command(subcommand)]
    resource: Resource,
}

/// The resources.
#[derive(clap::Subcommand)]
enum Resource {
    /// The operators a query can use: list them, or describe one.
    #[command(subcommand)]
    Operator(operator::OperatorCommand),
    /// RyoQL queries: check one, or run it over a repository.
    #[command(subcommand)]
    Query(query::QueryCommand),
    /// The relations a query can read: list them, or describe one.
    #[command(subcommand)]
    Relation(relation::RelationCommand),
}

/// The command tree, which the generated command-line contract test walks.
#[must_use]
pub fn command() -> clap::Command {
    // The format is the command line's, taken anywhere on it.
    ["format", "json", "quiet"]
        .into_iter()
        .fold(<Cli as clap::CommandFactory>::command(), |tree, id| {
            tree.mut_arg(id, |argument| argument.global(true))
        })
}

/// The directory beneath a project's root where Ryo keeps its store: written here once, and handed to every command by the edge,
/// which hands it on to the store and to the walks that skip it.
#[must_use]
fn store() -> ryo_core::outbound::lint_store::types::CacheDirectory {
    ryo_core::outbound::lint_store::types::CacheDirectory(".ryo_cache".into())
}

/// The edge of a command that looks for its project from `start`, run from `here`: the one place that builds an
/// [`Edge`](crate::edge::Edge), so the store's name and the build stay written once.
#[must_use]
pub fn edge_at(
    start: ryo_language::inbound::use_cases::project::types::RootPath,
    here: ryo_language::inbound::use_cases::project::types::RootPath,
    format: crate::output::Format,
) -> crate::edge::Edge {
    crate::edge::Edge {
        start,
        here,
        store: store(),
        build: ryo_core::outbound::lint_store::types::BuildIdentity(
            concat!(env!("CARGO_PKG_NAME"), " ", env!("CARGO_PKG_VERSION")).into(),
        ),
        format,
    }
}

/// Parses the command line and runs the command it names; a panic is reported as an
/// internal error and ends as one.
#[must_use]
pub fn run(here: String) -> std::process::ExitCode {
    report_panics();
    let outcome = std::panic::catch_unwind(|| run_from(std::env::args_os().collect(), here))
        .unwrap_or(crate::outcome::Outcome::Error);
    crate::output::progress::settle();
    std::process::ExitCode::from(outcome)
}

/// Runs the command line `arguments`, the program's name first.
fn run_from(arguments: Vec<std::ffi::OsString>, here: String) -> crate::outcome::Outcome {
    let started = std::time::Instant::now();
    color(&arguments);
    crate::output::quiet(&arguments);
    let matches = match command().try_get_matches_from(arguments) {
        Ok(matches) => matches,
        Err(error) => return usage(&error),
    };
    let cli = match <Cli as clap::FromArgMatches>::from_arg_matches(&matches) {
        Ok(cli) => cli,
        Err(error) => return usage(&error),
    };
    let _timings = match crate::timings::install(cli.timings.timings, started) {
        Ok(guard) => guard,
        Err(error) => return crate::output::fail(&error, crate::outcome::Outcome::Error),
    };
    let here = ryo_language::inbound::use_cases::project::types::RootPath(here.into());
    let start = cli.root.map_or_else(
        || here.clone(),
        |root| ryo_language::inbound::use_cases::project::types::RootPath(root.into()),
    );
    let edge = edge_at(start, here, cli.output.format());
    match cli.resource {
        Resource::Operator(command) => operator::run(command, &edge),
        Resource::Query(command) => query::run(command, &edge),
        Resource::Relation(command) => relation::run(command, &edge),
    }
}

/// Where to report a problem, printed under the top-level help.
const SUPPORT: &str = concat!(
    "Report a problem: ",
    env!("CARGO_PKG_REPOSITORY"),
    "/issues"
);

/// `--no-color`, which every command takes: no styles, wherever the output goes.
#[must_use]
fn no_color() -> clap::Arg {
    clap::Arg::new("no-color")
        .long("no-color")
        .global(true)
        .action(clap::ArgAction::SetTrue)
        .help("Print no styles, wherever the output goes, as `NO_COLOR` does")
}

/// Drops every style before anything is written when `arguments` hold `--no-color`
/// before any `--`: clap prints help and usage errors before the switch is parsed.
fn color(arguments: &[std::ffi::OsString]) {
    if arguments
        .iter()
        .take_while(|argument| *argument != "--")
        .any(|argument| argument == "--no-color")
    {
        anstream::ColorChoice::Never.write_global();
    }
}

/// Replaces Rust's panic message with what happened, where, and where to report it, on
/// stderr; the command then ends as [`crate::outcome::Outcome::Error`], never with Rust's 101.
fn report_panics() {
    std::panic::set_hook(Box::new(crate::output::panicked));
}

/// Prints clap's own answer, help, the version or a usage error, on the stream clap
/// chooses, and ends as clap does: help and the version as [`crate::outcome::Outcome::Success`], a usage
/// error as [`crate::outcome::Outcome::Error`]. A reader that closed the pipe does not make help fail.
fn usage(error: &clap::Error) -> crate::outcome::Outcome {
    let outcome = if error.use_stderr() {
        crate::outcome::Outcome::Error
    } else {
        crate::outcome::Outcome::Success
    };
    match error.print() {
        Err(failed) if failed.kind() != std::io::ErrorKind::BrokenPipe => {
            crate::outcome::Outcome::Error
        }
        _ => outcome,
    }
}
