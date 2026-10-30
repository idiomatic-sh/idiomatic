//! Proves `-n/--dry-run`: what a command that writes would write, and nothing written.

use super::{DryRunSwitch, deliver};
use crate::output::Format;

/// Reads a command line into the switch.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    switch: DryRunSwitch,
}

/// Without the flag a command writes.
#[test]
fn switch_succeeds_writing_by_default() {
    let parsed = <Line as clap::Parser>::try_parse_from(["asymptote"]).expect("a command line");
    assert!(!parsed.switch.dry_run());
}

/// `-n` and `--dry-run` both ask for a dry run.
#[test]
fn switch_succeeds_dry_with_either_spelling() {
    for flag in ["-n", "--dry-run"] {
        let parsed =
            <Line as clap::Parser>::try_parse_from(["asymptote", flag]).expect("a command line");
        assert!(parsed.switch.dry_run(), "{flag}");
    }
}

/// The help says the plan is printed on stderr, as `would <change>` lines.
#[test]
fn switch_succeeds_naming_stderr_and_the_lines_it_prints() {
    let command = <Line as clap::CommandFactory>::command();
    let help = command
        .get_arguments()
        .find(|argument| argument.get_long() == Some("dry-run"))
        .and_then(clap::Arg::get_help)
        .expect("the switch's help")
        .to_string();
    assert!(help.contains("on stderr"), "{help}");
    assert!(help.contains("`would record …`"), "{help}");
}

/// A plan in text is written to the stream it is handed, which `present` hands stderr, and unstyled in markdown.
#[test]
fn deliver_succeeds_writing_the_plan_whole() {
    let plan = "would write asymptote.toml and an empty lock\nwould write the pre-commit hook\n";
    for format in [Format::Terminal, Format::Markdown] {
        let mut stderr = Vec::new();
        deliver(format, &plan, &mut stderr).expect("the plan is written");
        assert_eq!(String::from_utf8(stderr).expect("text"), plan, "{format:?}");
    }
}

/// A value after the flag is refused: the switch takes none.
#[test]
fn switch_refuses_a_value() {
    assert!(<Line as clap::Parser>::try_parse_from(["asymptote", "--dry-run=yes"]).is_err());
}
