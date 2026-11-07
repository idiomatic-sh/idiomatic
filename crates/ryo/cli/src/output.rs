//! How a command's answer is printed: one `--format` switch, the one place an answer is
//! written. A command hands over its answer and never knows the format: json is
//! what its `schemas.rs` writes ([`Encoded`]), markdown is its text, and terminal is its text, styled
//! with anstyle where the answer styles it and written through anstream, which keeps
//! the styles where stdout takes them: a terminal, unless `--no-color`, `NO_COLOR` or
//! `CLICOLOR` says otherwise, as cargo, uv and ruff do. The variables are anstream's and
//! clap's to read. On a terminal the text goes through the user's pager, `PAGER` or
//! `less -FRX`, as git, gh and jj page; `PAGER` is the one variable this module reads, the
//! `pager` exception `no-env-reads` names.
//!
//! Beside the answer, on stderr: an error with what to do next ([`fail`],
//! `crate::outcome::remedy`), one line of progress while a long command works
//! ([`progress`]), and what a `--dry-run` ([`dry_run`]) would have written (`dry_run::planned`);
//! `--quiet` keeps only the error.
//!
//! The rows of a query's answer reach the command line typed; `schemas` is the one place a cell becomes text or JSON, as
//! each row is written.

pub mod dry_run;
mod pager;
pub mod progress;
pub(crate) mod schemas;

#[cfg(test)]
#[path = "output_test.rs"]
mod tests;

use std::io::Write as _;

use crate::outcome::CommandOutcome;

/// The formats an answer is printed in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, clap::ValueEnum)]
pub enum Format {
    /// Text for a person, styled where stdout takes styles.
    #[default]
    Terminal,
    /// One JSON document, for a program to read.
    Json,
    /// The same text as markdown, never styled.
    Markdown,
    /// Nothing: `--quiet`, where the exit code is the answer.
    #[value(skip)]
    Quiet,
}

/// The `--format` switch every command flattens into its arguments, with `--json` and
/// `-q/--quiet`, the spellings other command lines use.
#[derive(Clone, Copy, Debug, clap::Args)]
pub struct FormatSwitch {
    /// The format the answer is printed in.
    #[arg(long, value_enum, default_value_t)]
    format: Format,
    /// Print the answer as one JSON document: `--format json`.
    #[arg(long, conflicts_with = "format")]
    json: bool,
    /// Print no answer; the exit code says how the command ended, and errors still go to stderr.
    #[arg(short, long, conflicts_with_all = ["format", "json"])]
    quiet: bool,
}

impl FormatSwitch {
    /// The format chosen.
    #[must_use]
    pub const fn format(self) -> Format {
        if self.quiet {
            Format::Quiet
        } else if self.json {
            Format::Json
        } else {
            self.format
        }
    }
}

/// How an answer's JSON is spaced: on one line for a program, or indented for a person reading a log.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spacing {
    /// One line.
    Compact,
    /// Indented, two spaces a level.
    Indented,
}

/// An answer a command prints: its text, and its JSON, which the command's own `schemas.rs` writes, the one place its
/// shape is encoded.
pub trait Encoded: std::fmt::Display {
    /// Writes the answer's JSON to `to`, spaced as `spacing` says.
    ///
    /// # Errors
    ///
    /// When `to` refuses the bytes, or the answer holds a value JSON cannot write.
    fn json(&self, to: &mut dyn std::io::Write, spacing: Spacing) -> std::io::Result<()>;
}

/// Prints `answer` on stdout in `format` and ends the command as `outcome`, or as
/// [`CommandOutcome::Error`] when stdout refuses it: the answer never reached its reader. A
/// reader that closed the pipe (`| head`) took what it wanted, so the outcome stands.
pub fn present(format: Format, answer: &impl Encoded, outcome: CommandOutcome) -> CommandOutcome {
    progress::settle();
    match print(format, answer) {
        Err(failed) if failed.kind() != std::io::ErrorKind::BrokenPipe => unwritten(&failed),
        _ => outcome,
    }
}

/// Streams the answer `body` writes in `format`: through the pager on a terminal, to stdout
/// otherwise, and nowhere under `--quiet`. The body owns the stream and writes to it as it
/// pleases: row by row, or, for an answer that must be one whole document, once at the end. Ends as [`CommandOutcome::Error`] when the pager cannot be waited
/// for, and as `outcome` otherwise.
pub fn stream(
    format: Format,
    outcome: CommandOutcome,
    body: impl FnOnce(Box<dyn std::io::Write + Send>),
) -> CommandOutcome {
    progress::settle();
    let delivered = match format {
        Format::Quiet => {
            body(Box::new(std::io::sink()));
            Ok(())
        }
        Format::Terminal => pager::page_with(body),
        Format::Json | Format::Markdown => {
            body(Box::new(std::io::BufWriter::new(anstream::stdout())));
            Ok(())
        }
    };
    match delivered {
        Err(failed) if failed.kind() != std::io::ErrorKind::BrokenPipe => unwritten(&failed),
        _ => outcome,
    }
}

/// The bytes a run writes while it works, held to be written when it ends well: a run that stops on the way leaves stdout without a
/// prefix of a document.
#[derive(Clone, Debug, Default)]
struct Held(std::sync::Arc<std::sync::Mutex<Vec<u8>>>);

impl std::io::Write for Held {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

/// Writes to `out` the answer `body` produces, in the way `format` needs: as JSON, one complete document written only when `body`
/// ends well, so that a run that stops leaves no part of a document behind; row by row, as `body` writes them, in the other formats.
///
/// # Errors
///
/// `body`'s own failure, or `unwritable` of the error `out` refused the document with.
pub fn written<E>(
    format: Format,
    mut out: Box<dyn std::io::Write + Send>,
    body: impl FnOnce(Box<dyn std::io::Write + Send>) -> Result<(), E>,
    unwritable: impl FnOnce(&std::io::Error) -> E,
) -> Result<(), E> {
    if format != Format::Json {
        return body(out);
    }
    let held = Held::default();
    body(Box::new(held.clone()))?;
    let document = std::mem::take(
        &mut *held
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner),
    );
    out.write_all(&document)
        .and_then(|()| out.flush())
        .map_err(|error| unwritable(&error))
}

/// Ends a command whose answer stdout `failed` to take, as [`CommandOutcome::Error`]: the answer
/// never reached its reader, which stderr says with what to do next.
pub fn unwritten(failed: &std::io::Error) -> CommandOutcome {
    // Nothing is left to say when stderr is gone; the exit code still says it.
    let _unused = writeln!(
        anstream::stderr(),
        "error: could not write the answer to stdout: {failed}\nhelp: give the command a stdout that takes its answer (a full disk or a closed file refuses it), or pass `--quiet`"
    );
    CommandOutcome::Error
}

/// Says why the command could not answer, `error` with each cause beneath it once
/// (`error: cause: cause`), then what to do next (`help: …`), on stderr in every format,
/// and ends the command as `outcome`, unless stderr refuses delivery.
pub fn fail(
    error: &(impl std::error::Error + crate::outcome::remedy::Remedy),
    outcome: CommandOutcome,
) -> CommandOutcome {
    progress::settle();
    let mut message = error.to_string();
    let mut cause = error.source();
    while let Some(source) = cause {
        message.push_str(": ");
        message.push_str(&source.to_string());
        cause = source.source();
    }
    match writeln!(
        anstream::stderr().lock(),
        "error: {message}\nhelp: {}",
        error.next()
    ) {
        Ok(()) => outcome,
        Err(_) => CommandOutcome::Error,
    }
}

/// Says that the command panicked: what happened, where, and where to report it, on stderr,
/// in place of Rust's own text; the command then ends as [`CommandOutcome::Error`].
pub fn panicked(panic: &std::panic::PanicHookInfo<'_>) {
    let place = panic
        .location()
        .map(|location| format!(" at {}:{}", location.file(), location.line()))
        .unwrap_or_default();
    let message = panic
        .payload()
        .downcast_ref::<&str>()
        .map(|message| (*message).to_owned())
        .or_else(|| panic.payload().downcast_ref::<String>().cloned())
        .unwrap_or_default();
    // Nothing is left to say when stderr is gone.
    let _unused = writeln!(
        std::io::stderr(),
        "error: internal error in {} {}{place}: {message}\nhelp: this is a bug; please report it, with the command line you ran: {}/issues/new",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY"),
    );
}

/// Drops progress and planned changes when `arguments` hold `--quiet` or `-q`, alone or
/// among other one-letter flags, before any `--`: they are said by the services a
/// command composes, which never see the command's format.
pub fn quiet(arguments: &[std::ffi::OsString]) {
    let asked = arguments
        .iter()
        .skip(1)
        .take_while(|argument| *argument != "--")
        .filter_map(|argument| argument.to_str())
        .any(|argument| {
            argument == "--quiet"
                || argument
                    .strip_prefix('-')
                    .is_some_and(|letters| !letters.starts_with('-') && letters.contains('q'))
        });
    if asked {
        progress::QUIET.store(true, std::sync::atomic::Ordering::Relaxed);
    }
}

/// Prints `answer` on stdout in `format`.
fn print(format: Format, answer: &impl Encoded) -> std::io::Result<()> {
    if format == Format::Terminal {
        return pager::page(answer);
    }
    let mut stdout = anstream::stdout().lock();
    match format {
        Format::Terminal | Format::Quiet => Ok(()),
        Format::Json => answer
            .json(&mut stdout, Spacing::Compact)
            .and_then(|()| writeln!(stdout)),
        Format::Markdown => stdout.write_all(markdown(answer).as_bytes()),
    }?;
    stdout.flush()
}

/// The answer's text as markdown: its text with any style dropped.
fn markdown(answer: &impl std::fmt::Display) -> String {
    anstream::adapter::strip_str(&answer.to_string()).to_string()
}
