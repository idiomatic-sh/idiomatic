//! What the schemas of `down` make of its arguments, a dry run or a write, and how its answer says a perf record it
//! drops.

use clap::error::ErrorKind;

use super::super::command::DownArguments;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The command line, after `down`; or, empty and `perf`, the JSON of a perf record dropped.
type Input = Vec<&'static str>;
/// Whether the request writes; or the JSON of the perf records dropped.
type Value = std::borrow::Cow<'static, str>;
/// The kind of error the command line parser refused the line with.
type Failure = ErrorKind;

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    /// The arguments the line parses into.
    #[command(flatten)]
    arguments: DownArguments,
    /// The dry run the line chose, which the command tree reads into the edge.
    #[command(flatten)]
    dry_run: crate::output::dry_run::DryRunSwitch,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_perf_record_dropped_is_its_key_in_the_json"),
        word: Word::Succeeds,
        input: || vec!["perf"],
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            r#"[{"benchmark":"check","measure":"wall-ms","tool":"bench"}]"#,
        )),
    },
    Case {
        name: CaseName("no_flag_writes"),
        word: Word::Succeeds,
        input: Vec::new,
        expect: Expect::Value(std::borrow::Cow::Borrowed("writes")),
    },
    Case {
        name: CaseName("a_dry_run_does_not"),
        word: Word::Succeeds,
        input: || vec!["-n"],
        expect: Expect::Value(std::borrow::Cow::Borrowed("dry")),
    },
    Case {
        name: CaseName("a_selector_is_refused_down_drains_every_fixed_record"),
        word: Word::Refuses,
        input: || vec!["--rule", "r"],
        expect: Expect::error(ErrorKind::UnknownArgument),
    },
];

/// Whether the request of the row's command line writes.
fn subject(line: Input) -> Result<Value, Failure> {
    if line == ["perf"] {
        return Ok(std::borrow::Cow::Owned(perf_json()));
    }
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("down").chain(line))
        .map_err(|error| error.kind())?;
    let root = asymptote_core::domain::manifest::types::ProductRoot("/repo".into());
    let edge = crate::edge::Edge {
        here: root.clone(),
        start: root,
        store: std::path::PathBuf::from(asymptote_core::domain::operations::constants::CACHE),
        build: asymptote_core::outbound::check_store::types::Build("test".into()),
        format: crate::output::Format::Terminal,
        writing: if parsed.dry_run.dry_run() {
            asymptote_core::domain::ratchet::types::Writing::DryRun
        } else {
            asymptote_core::domain::ratchet::types::Writing::Write
        },
    };
    let request = super::request(&edge);
    Ok(std::borrow::Cow::Borrowed(
        if request.writing == asymptote_core::domain::ratchet::types::Writing::Write {
            "writes"
        } else {
            "dry"
        },
    ))
}

/// The JSON `down` answers with for one perf record dropped.
fn perf_json() -> String {
    use asymptote_core::domain::operations::types::{
        BenchmarkName, MeasureName, Number, PerfKey, PerfRecord, Reason,
    };
    use asymptote_core::domain::ratchet::types::{Compaction, Writing};
    let report = asymptote_core::inbound::use_cases::down::types::DownReport {
        root: asymptote_core::domain::manifest::types::ProductRoot("/repo".into()),
        dropped: Vec::new(),
        lowered: Vec::new(),
        recorded: Vec::new(),
        perf: vec![PerfRecord {
            key: PerfKey {
                tool: asymptote_core::domain::violations::types::ToolId("bench".into()),
                benchmark: BenchmarkName("check".into()),
                measure: MeasureName("wall-ms".into()),
            },
            best: Number(1.0),
            reasons: vec![Reason("seeded".into())],
        }],
        compaction: Compaction::Unchanged,
        writing: Writing::Write,
    };
    serde_json::to_value(super::DownReportSchema::from(&report))
        .ok()
        .and_then(|json| json.get("perf").map(ToString::to_string))
        .unwrap_or_default()
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
