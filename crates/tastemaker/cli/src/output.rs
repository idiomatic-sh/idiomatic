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
//! ([`progress`]), what a `--dry-run` ([`dry_run`]) would have written (`dry_run::planned`),
//! and a warning a service gave of something that did not stop the command (`warnings`);
//! `--quiet` keeps only the error and the warnings.

pub mod dry_run;
mod pager;
pub mod progress;

#[cfg(test)]
#[path = "output_test.rs"]
mod tests;

use std::io::Write as _;

use crate::outcome::Outcome;

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
    /// A SARIF 2.1.0 log, for a ratchet to read: `check` prints its findings as one; any other command prints JSON.
    Sarif,
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

/// The answer's JSON as `--format json` prints it, for a test to read.
///
/// # Errors
///
/// When the answer cannot be written as JSON.
#[cfg(test)]
pub(crate) fn json_text(answer: &impl Encoded) -> std::io::Result<String> {
    let mut written = Vec::new();
    answer.json(&mut written, Spacing::Compact)?;
    String::from_utf8(written).map_err(std::io::Error::other)
}

/// Prints `answer` on stdout in `format` and ends the command as `outcome`, or as
/// [`Outcome::Error`] when stdout refuses it: the answer never reached its reader. A
/// reader that closed the pipe (`| head`) took what it wanted, so the outcome stands.
pub fn present(format: Format, answer: &impl Encoded, outcome: Outcome) -> Outcome {
    progress::settle();
    match print(format, answer) {
        Err(failed) if failed.kind() != std::io::ErrorKind::BrokenPipe => unwritten(&failed),
        _ => outcome,
    }
}

/// Prints a command's answer as `present` does, but a plan, what a command that writes would have written, on stderr
/// under `-n/--dry-run`: stdout holds what a command answers, never what it only says it would do.
pub fn present_planned(
    format: Format,
    writing: tastemaker_core::domain::writing::types::Writing,
    answer: &impl Encoded,
    outcome: Outcome,
) -> Outcome {
    if writing != tastemaker_core::domain::writing::types::Writing::DryRun {
        return present(format, answer, outcome);
    }
    progress::settle();
    match plan(format, answer) {
        Err(failed) if failed.kind() != std::io::ErrorKind::BrokenPipe => unwritten(&failed),
        _ => outcome,
    }
}

/// Streams the answer `body` writes in `format`: through the pager on a terminal, to stdout
/// otherwise, and nowhere under `--quiet`. The body writes as it produces and owns the stream;
/// nothing holds the whole answer. Ends as [`Outcome::Error`] when the pager cannot be waited
/// for, and as `outcome` otherwise.
pub fn stream(
    format: Format,
    outcome: Outcome,
    body: impl FnOnce(Box<dyn std::io::Write + Send>),
) -> Outcome {
    progress::settle();
    let delivered = match format {
        Format::Quiet => {
            body(Box::new(std::io::sink()));
            Ok(())
        }
        Format::Terminal => pager::page_with(body),
        Format::Json | Format::Markdown | Format::Sarif => {
            body(Box::new(std::io::BufWriter::new(anstream::stdout())));
            Ok(())
        }
    };
    match delivered {
        Err(failed) if failed.kind() != std::io::ErrorKind::BrokenPipe => unwritten(&failed),
        _ => outcome,
    }
}

/// Ends a command whose answer stdout `failed` to take, as [`Outcome::Error`]: the answer
/// never reached its reader, which stderr says with what to do next.
pub fn unwritten(failed: &std::io::Error) -> Outcome {
    // Nothing is left to say when stderr is gone; the exit code still says it.
    drop(writeln!(
        anstream::stderr(),
        "error: could not write the answer to stdout: {failed}\nhelp: give the command a stdout that takes its answer (a full disk or a closed file refuses it), or pass `--quiet`"
    ));
    Outcome::Error
}

/// `error` with each of its causes after it, `error: cause: cause`, a cause its error's text already says left out, so
/// none is said twice.
fn said(error: &dyn std::error::Error) -> String {
    let mut message = error.to_string();
    let mut cause = error.source();
    while let Some(source) = cause {
        let text = source.to_string();
        if !message.contains(&text) {
            message.push_str(": ");
            message.push_str(&text);
        }
        cause = source.source();
    }
    message
}

/// Says why the command could not answer, `error` with each cause beneath it once
/// (`error: cause: cause`), then what to do next (`help: …`), on stderr in every format,
/// and ends the command as `outcome`, unless stderr refuses delivery.
pub fn fail(
    error: &(impl std::error::Error + crate::outcome::remedy::Remedy),
    outcome: Outcome,
) -> Outcome {
    progress::settle();
    let message = said(error);
    match writeln!(
        anstream::stderr().lock(),
        "error: {message}\nhelp: {}",
        error.next()
    ) {
        Ok(()) => outcome,
        Err(_) => Outcome::Error,
    }
}

/// The tracing layer that says each warning the core gives on stderr, as `warning: <message>`
/// (`tracing::warn!(message = %error)`): something went wrong that did not change the command's
/// answer or its exit code, such as a cache that could not be saved. The `timings` module adds it
/// to the subscriber it installs.
#[must_use]
pub(crate) fn warnings<S>() -> impl tracing_subscriber::Layer<S>
where
    S: tracing::Subscriber + for<'span> tracing_subscriber::registry::LookupSpan<'span>,
{
    tracing_subscriber::Layer::with_filter(
        Warnings {
            out: std::sync::Mutex::new(anstream::stderr()),
        },
        tracing_subscriber::filter::Targets::new()
            .with_target("tastemaker_core", tracing::Level::WARN),
    )
}

/// Says each warning's message, one line each, on `out`: stderr, or a buffer in a test.
struct Warnings<W> {
    /// Where the lines go.
    out: std::sync::Mutex<W>,
}

impl<S: tracing::Subscriber, W: std::io::Write + Send + 'static> tracing_subscriber::Layer<S>
    for Warnings<W>
{
    fn on_event(
        &self,
        event: &tracing::Event<'_>,
        _context: tracing_subscriber::layer::Context<'_, S>,
    ) {
        let mut message = progress::Message::default();
        event.record(&mut message);
        progress::settle();
        let mut out = self
            .out
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        // Nothing is left to say when stderr is gone; the answer and the exit code stand.
        drop(writeln!(out, "warning: {}", message.into_text()));
    }
}

/// Says that the command panicked: what happened, where, and where to report it, on stderr,
/// in place of Rust's own text; the command then ends as [`Outcome::Error`].
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
    drop(writeln!(
        std::io::stderr(),
        "error: internal error in {} {}{place}: {message}\nhelp: this is a bug; please report it, with the command line you ran: {}/issues/new",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_PKG_REPOSITORY"),
    ));
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
        Format::Sarif => answer
            .json(&mut stdout, Spacing::Indented)
            .and_then(|()| writeln!(stdout)),
        Format::Markdown => stdout.write_all(markdown(answer).as_bytes()),
    }?;
    stdout.flush()
}

/// Prints `answer` on stderr in `format`: nothing under `--quiet`, and never through the pager.
fn plan(format: Format, answer: &impl Encoded) -> std::io::Result<()> {
    let mut stderr = anstream::stderr().lock();
    match format {
        Format::Quiet => return Ok(()),
        Format::Terminal => write!(stderr, "{answer}"),
        Format::Json => answer
            .json(&mut stderr, Spacing::Compact)
            .and_then(|()| writeln!(stderr)),
        Format::Sarif => answer
            .json(&mut stderr, Spacing::Indented)
            .and_then(|()| writeln!(stderr)),
        Format::Markdown => stderr.write_all(markdown(answer).as_bytes()),
    }?;
    stderr.flush()
}

/// The answer's text as markdown: its text with any style dropped.
fn markdown(answer: &impl std::fmt::Display) -> String {
    anstream::adapter::strip_str(&answer.to_string()).to_string()
}
