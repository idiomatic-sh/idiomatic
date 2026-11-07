//! `-n/--dry-run` (clig.dev, Arguments and flags): what a command that writes would write,
//! and nothing written.

use std::io::Write as _;

#[cfg(test)]
#[path = "dry_run_test.rs"]
mod tests;

/// `-n/--dry-run`, which the command line takes anywhere on it, for every command that
/// writes (the `cli` spec's `writes`): the command runs against a planned adapter, which
/// writes nothing and says each change it would have made ([`planned`]).
#[derive(Clone, Copy, Debug, Default, clap::Args)]
pub struct DryRunSwitch {
    /// Write nothing; print on stderr what would be written (`would write …`, `would record …`, `would drain …`).
    #[arg(short = 'n', long)]
    dry_run: bool,
}

impl DryRunSwitch {
    /// Whether the command only says what it would write.
    #[must_use]
    pub const fn dry_run(self) -> bool {
        self.dry_run
    }
}

/// Presents the answer of a command that writes: as any answer is when it wrote; under `--dry-run` its plan, as
/// text on stderr, as the switch's help says, as one JSON document on stdout under `--json`, for a program to read,
/// and nowhere under `--quiet`. Ends as `outcome`, or as the answer's failure when stdout refuses it.
pub fn present(
    writing: asymptote_core::domain::ratchet::types::Writing,
    format: super::Format,
    answer: &impl super::Encoded,
    outcome: crate::outcome::Outcome,
) -> crate::outcome::Outcome {
    use asymptote_core::domain::ratchet::types::Writing;
    super::progress::settle();
    match (writing, format) {
        (Writing::Write, _) | (Writing::DryRun, super::Format::Json | super::Format::Quiet) => {
            super::present(format, answer, outcome)
        }
        (
            Writing::DryRun,
            super::Format::Terminal | super::Format::Markdown | super::Format::Sarif,
        ) => {
            // Nothing is left to say when stderr is gone; the exit code still says how the command ended.
            drop(deliver(format, answer, &mut anstream::stderr().lock()));
            outcome
        }
    }
}

/// Writes a dry run's plan, `answer`, in the text `format` asks for to `stderr`: styled where it takes styles in
/// the terminal format, plain otherwise.
fn deliver(
    format: super::Format,
    answer: &dyn std::fmt::Display,
    stderr: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    match format {
        super::Format::Terminal => write!(stderr, "{answer}"),
        super::Format::Markdown
        | super::Format::Sarif
        | super::Format::Json
        | super::Format::Quiet => {
            write!(
                stderr,
                "{}",
                anstream::adapter::strip_str(&answer.to_string())
            )
        }
    }
}

/// Says one change a `--dry-run` would have made, `would <change>`, on stderr; nothing
/// under `--quiet`. The planned adapters a command composes under `--dry-run` call it.
pub fn planned(change: &dyn std::fmt::Display) {
    if super::progress::QUIET.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    super::progress::settle();
    // Nothing is left to say when stderr is gone.
    drop(writeln!(anstream::stderr(), "would {change}"));
}
