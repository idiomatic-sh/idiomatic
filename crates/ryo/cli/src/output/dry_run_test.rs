//! Proves `-n/--dry-run`: what a command that writes would write, and nothing written.

use super::DryRunSwitch;

/// Reads a command line into the switch.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    switch: DryRunSwitch,
}

/// Without the flag a command writes.
#[test]
fn switch_succeeds_writing_by_default() {
    let parsed = <Line as clap::Parser>::try_parse_from(["ryo"]).expect("a command line");
    assert!(!parsed.switch.dry_run());
}

/// `-n` and `--dry-run` both ask for a dry run.
#[test]
fn switch_succeeds_dry_with_either_spelling() {
    for flag in ["-n", "--dry-run"] {
        let parsed = <Line as clap::Parser>::try_parse_from(["ryo", flag]).expect("a command line");
        assert!(parsed.switch.dry_run(), "{flag}");
    }
}

/// A value after the flag is refused: the switch takes none.
#[test]
fn switch_refuses_a_value() {
    assert!(<Line as clap::Parser>::try_parse_from(["ryo", "--dry-run=yes"]).is_err());
}
