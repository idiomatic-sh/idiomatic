//! `-n/--dry-run` (clig.dev, Arguments and flags): what a command that writes would write,
//! and nothing written.

use std::io::Write as _;

#[cfg(test)]
#[path = "dry_run_test.rs"]
mod tests;

/// `-n/--dry-run`, which every command that writes flattens into its arguments (the
/// `cli` spec's `writes`): the command runs against a planned adapter, which writes
/// nothing and says each change it would have made ([`planned`]).
#[derive(Clone, Copy, Debug, Default, clap::Args)]
pub struct DryRunSwitch {
    /// Write nothing; print what would be written, `would write <path>`, on stderr.
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

/// Says one change a `--dry-run` would have made, `would <change>`, on stderr; nothing
/// under `--quiet`. The planned adapters a command composes under `--dry-run` call it.
pub fn planned(change: &dyn std::fmt::Display) {
    if super::progress::QUIET.load(std::sync::atomic::Ordering::Relaxed) {
        return;
    }
    super::progress::settle();
    // Nothing is left to say when stderr is gone.
    let _unused = writeln!(anstream::stderr(), "would {change}");
}
