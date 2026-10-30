//! What the schemas of `check` accept and refuse, and the SARIF log they make: every result an error, suppressions and
//! fingerprints as the first Tastemaker wrote them.

use tastemaker_core::domain::files::types::MachinePath;
use tastemaker_core::domain::findings::types::{
    Column, Finding, Fingerprint, Help, Line, MessageText, Place, Reason, Region, Report, RuleId,
    Suppression,
};
use tastemaker_core::domain::paths::types::RepoPath;
use tastemaker_core::inbound::use_cases::check::types::{CheckReport, LockOutcome};

use super::answer;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outcome::remedy::{Next, Remedy as _};
use crate::output::Format;

/// What a row gives the schemas: a finding, accepted by an ignore or not, or a command line whose path is not text.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A finding, and whether an ignore accepts it.
    Finding(bool),
    /// Two findings of one rule whose help is not the same text, as a help that names what was found is.
    Interpolated,
    /// A path that is not text. A Unix path from raw bytes and a Windows path from a lone surrogate can be other than text,
    /// so the row and this variant exist there.
    #[cfg(any(unix, windows))]
    PathNotText,
}

/// What a row gives the schemas.
type Input = Scenario;
/// The SARIF level of its result, or none when it is no result.
type Value = Option<String>;
/// What the user is told to do next about the refusal.
type Failure = Next;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_error_is_an_error_result"),
        word: Word::Succeeds,
        input: || Scenario::Finding(false),
        expect: Expect::Value(Some(String::new())),
    },
    Case {
        name: CaseName("an_ignored_finding_is_a_suppressed_error_result"),
        word: Word::Succeeds,
        input: || Scenario::Finding(true),
        expect: Expect::Value(Some(String::new())),
    },
    Case {
        name: CaseName("a_rule_whose_help_names_what_was_found_has_none_of_its_own"),
        word: Word::Succeeds,
        input: || Scenario::Interpolated,
        expect: Expect::Value(Some(String::new())),
    },
    #[cfg(any(unix, windows))]
    Case {
        name: CaseName("a_path_that_is_not_text_is_refused_and_told_to_be_named_in_utf_8"),
        word: Word::Refuses,
        input: || Scenario::PathNotText,
        expect: Expect::Refusal(|next| *next == Next::Fix("name the path in UTF-8".to_owned())),
    },
];

/// A path that is not text.
#[cfg(unix)]
fn not_text() -> std::ffi::OsString {
    use std::os::unix::ffi::OsStringExt as _;
    std::ffi::OsString::from_vec(vec![b'a', 0xff, 0xfe])
}

/// A path that is not text: a lone surrogate, which no UTF-8 holds.
#[cfg(windows)]
fn not_text() -> std::ffi::OsString {
    use std::os::windows::ffi::OsStringExt;
    std::ffi::OsString::from_wide(&[u16::from(b'a'), 0xD800])
}

/// Refuses the command line whose path is not text, and says what to do instead.
#[cfg(any(unix, windows))]
fn refused() -> Result<Value, Failure> {
    use super::super::command::CheckArguments;

    /// Reads a command line into this command's arguments.
    #[derive(clap::Parser)]
    struct Line {
        #[command(flatten)]
        arguments: CheckArguments,
    }
    let parsed =
        <Line as clap::Parser>::try_parse_from([std::ffi::OsString::from("check"), not_text()])
            .map_err(|_error| Next::Report)?;
    let edge = crate::resources::edge_at(
        MachinePath("/repo".into()),
        tastemaker_core::domain::settings::types::Build("test".into()),
        Format::Terminal,
        tastemaker_core::domain::writing::types::Writing::Write,
    );
    let error = super::request(parsed.arguments.paths, parsed.arguments.fix, &edge)
        .err()
        .ok_or(Next::Report)?;
    Err(error.next())
}

fn finding() -> Finding {
    Finding {
        path: RepoPath("src/a b.rs".into()),
        place: Place::At(Region {
            start_line: Line(2),
            start_column: Column(1),
            end_line: Line(2),
            end_column: Column(4),
        }),
        rule: tastemaker_core::domain::findings::service::of_rule(RuleId("p/r".into())),
        message: MessageText("a {thing}".into()),
        help: Help::Said(MessageText("fix it".into())),
        fingerprint: Fingerprint(0xab),
    }
}

/// Two findings of one rule that say different things: no help is the rule's own, so `rules` names none and each finding says its own.
fn interpolated() -> Result<Value, Failure> {
    let mut other = finding();
    other.help = Help::Said(MessageText("fix that".into()));
    other.fingerprint = Fingerprint(0xcd);
    let mut report = Report {
        workspace: tastemaker_core::domain::findings::types::WorkspaceName("w".into()),
        ..Report::default()
    };
    report.findings.push(finding());
    report.findings.push(other);
    let checked = CheckReport {
        report,
        repository: MachinePath("/repo".into()),
        lock: LockOutcome::Current,
    };
    let json = serde_json::to_value(answer(&checked, Format::Json).printed)
        .map_err(|_error| Next::Report)?;
    let says = |place: usize| json["findings"][place]["help"].as_str().map(str::to_owned);
    Ok(Some(
        if json["rules"].get("p/r").is_none()
            && says(0).as_deref() == Some("fix it")
            && says(1).as_deref() == Some("fix that")
        {
            String::new()
        } else {
            json.to_string()
        },
    ))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let ignored = match scenario {
        Scenario::Finding(ignored) => ignored,
        Scenario::Interpolated => return interpolated(),
        #[cfg(any(unix, windows))]
        Scenario::PathNotText => return refused(),
    };
    let mut report = Report {
        workspace: tastemaker_core::domain::findings::types::WorkspaceName("w".into()),
        ..Report::default()
    };
    if ignored {
        report.suppressed.push(Suppression {
            finding: finding(),
            reason: Reason("legacy".into()),
        });
    } else {
        report.findings.push(finding());
    }
    let checked = CheckReport {
        report,
        repository: MachinePath("/repo".into()),
        lock: LockOutcome::Current,
    };
    // The JSON lists the finding with no level: every finding is an error.
    let json = serde_json::to_value(answer(&checked, Format::Json).printed)
        .map_err(|_error| Next::Report)?;
    let listed = if ignored {
        &json["suppressed"]
    } else {
        &json["findings"]
    };
    // The help of a rule is said once, under `rules`, and the finding that says the same carries none.
    if json["rules"]["p/r"] != "fix it" || listed[0].get("help").is_some() {
        return Ok(Some(json.to_string()));
    }
    // A field with nothing to say is left out: an unsuppressed finding has no `reason`.
    if listed[0].get("level").is_some()
        || json.get("counts").is_some()
        || listed[0].get("reason").is_some() != ignored
    {
        return Ok(Some(json.to_string()));
    }
    let sarif = answer(&checked, Format::Sarif);
    let log = serde_json::to_value(&sarif.printed).map_err(|_error| Next::Report)?;
    // The driver names the public repository and the version of the build that wrote the log.
    let driver = &log["runs"][0]["tool"]["driver"];
    if driver["informationUri"] != "https://github.com/idiomatic-sh/idiomatic"
        || driver["version"] != env!("CARGO_PKG_VERSION")
    {
        return Ok(Some(driver.to_string()));
    }
    let Some(result) = log["runs"][0]["results"].get(0) else {
        return Ok(None);
    };
    // SARIF's default level is `warning`, so every result writes `error`.
    let fine = result["level"] == "error"
        && result.get("partialFingerprints").is_none()
        && result["locations"][0]["physicalLocation"]["artifactLocation"]["uri"] == "src/a%20b.rs"
        && result["message"]["text"] == "a {{thing}}"
        && result["message"]["markdown"]
            .as_str()
            .is_some_and(|markdown| markdown.contains("{{thing}}"))
        && (result["suppressions"].as_array().map(Vec::len) == Some(usize::from(ignored)));
    Ok(Some(if fine {
        String::new()
    } else {
        result.to_string()
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
#[cfg(any(unix, windows))]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
