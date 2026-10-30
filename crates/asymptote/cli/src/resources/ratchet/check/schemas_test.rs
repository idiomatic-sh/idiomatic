// ignore: idiomatic.arch/file-length - the command spec places one schemas_test.rs beside check's one schemas.rs: its case table proves each schema family that file holds (the request, the perf lines, a workspace's answer, the groups and hooks refused, the lines said on stderr), and no second test file role exists.
//! What the schemas of `check` accept and refuse, and the lines its answer prints: the request its arguments become,
//! the perf lines, and what a workspace prints.

use asymptote_core::domain::judgement::types::{
    Code, Failure as RunFailure, Inventory, Run, Status, Tail, Verdict,
};
use asymptote_core::domain::manifest::types::{Group, ProductRoot};
use asymptote_core::domain::operations::types::{
    BenchmarkName, Count, MeasureName, Number, PerfKey, PerfRecord, Reason,
};
use asymptote_core::domain::perf::types::{
    Improvement, PerfOutcome, Regression, Rewriting, Unseeded,
};
use asymptote_core::domain::violations::types::ToolId;
use asymptote_core::inbound::use_cases::check::errors::CheckError;
use asymptote_core::inbound::use_cases::check::types::{
    Against, CheckReport, CheckRequest, FailedFix, Reuse, ToolRun,
};

use super::super::command::CheckArguments;
use super::{PerfSchema, answered};
use crate::cases::{Case, CaseName, Expect, Word, run};
use asymptote_core::domain::manifest::types::Layout;

/// Which question is asked.
#[derive(Clone, Debug)]
enum Question {
    /// The arguments after `check`, and what request they become.
    Line(Vec<&'static str>),
    /// The arguments after `check`, under `--dry-run`.
    Previewed(Vec<&'static str>),
    /// The perf outcome of a run, printed.
    Perf(Outcomes),
    /// What a product or a workspace prints for these verdicts, at these paths.
    Printed(Layout, Vec<(&'static str, Verdict)>),
    /// A product that could not answer.
    Unanswered,
    /// The JSON's `failed_fixes`: a fix that failed when true, none when false.
    FixJson(bool),
    /// What a refused `--hook X` says when the root maps these hooks.
    Refused(&'static str),
    /// What is said on stderr, `--quiet` included, for a tool that failed and a fix that failed, in this layout.
    Said(Layout),
    /// The arguments after `check`, held to the groups the tools declare (`lint`, `perf`).
    Known(Vec<&'static str>),
    /// The JSON of a run with a new violation an in-source ignore suppresses, a resolved record and an unjudged one.
    Judged,
}

/// The perf outcomes a run comes to.
#[derive(Clone, Copy, Debug)]
enum Outcomes {
    /// A median over its best.
    Regressed,
    /// A median with no record.
    Unseeded,
    /// A median under its best.
    Improved,
    /// A median under its best, in a run that leaves the lock as it is.
    ImprovedWithheld,
    /// A record the tool no longer reports.
    Resolved,
    /// Medians within their record.
    Held,
    /// Nothing at all.
    Nothing,
}

/// What a row asks of the schemas.
type Input = Question;
/// What the question answers.
type Value = std::borrow::Cow<'static, str>;
/// What a refusal says: the refused line's error, or the outcome an unanswered product ends the command as.
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "suppressed_findings_are_their_own_list_with_their_reason_and_records_keep_their_counts",
        ),
        word: Word::Succeeds,
        input: || Question::Judged,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            r#"[{"column":null,"count":1,"fingerprint":"00000000000000ff","level":"warning","line":null,"message":"m","path":"src/b.rs","remaining":1,"rule":"r","status":"resolved","tool":"lint"}] | [{"column":null,"fingerprint":"00000000000000ff","level":"warning","line":1,"message":"m","path":"src/a.rs","reason":"generated","rule":"r","status":"new","tool":"lint"}] | [{"column":null,"count":2,"fingerprint":"00000000000000ff","level":"warning","line":null,"message":"m","path":"src/b.rs","rule":"r","status":"accepted","tool":"lint","why":"tool-not-run"}]"#,
        )),
    },
    Case {
        name: CaseName("since_becomes_a_request_against_its_revision"),
        word: Word::Succeeds,
        input: || Question::Line(vec!["--since", "origin/main"]),
        expect: Expect::Value(std::borrow::Cow::Borrowed("since All")),
    },
    Case {
        name: CaseName("no_since_is_judged_against_the_lock"),
        word: Word::Succeeds,
        input: || Question::Line(Vec::new()),
        expect: Expect::Value(std::borrow::Cow::Borrowed("lock All")),
    },
    Case {
        name: CaseName("groups_are_comma_separated"),
        word: Word::Succeeds,
        input: || Question::Line(vec!["--groups", "lint,test"]),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "lock Only([Group(\"lint\"), Group(\"test\")])",
        )),
    },
    Case {
        name: CaseName("groups_may_be_repeated"),
        word: Word::Succeeds,
        input: || Question::Line(vec!["--groups", "perf", "--groups", "test"]),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "lock Only([Group(\"perf\"), Group(\"test\")])",
        )),
    },
    Case {
        name: CaseName("a_regression_names_the_benchmark_measure_value_and_best"),
        word: Word::Succeeds,
        input: || Question::Perf(Outcomes::Regressed),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "regressed: check/wall-ms 120 is over the best 100 (bench)\n|; perf: 1 regressed, 0 unseeded, 0 improved, 0 resolved, 0 held",
        )),
    },
    Case {
        name: CaseName("an_unseeded_median_says_how_to_seed_it"),
        word: Word::Succeeds,
        input: || Question::Perf(Outcomes::Unseeded),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "unseeded: check/wall-ms 120; seed with `asymptote up --reason <why> --tool bench`\n|; perf: 0 regressed, 1 unseeded, 0 improved, 0 resolved, 0 held",
        )),
    },
    Case {
        name: CaseName("an_improvement_says_to_commit_the_lock"),
        word: Word::Succeeds,
        input: || Question::Perf(Outcomes::Improved),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "improved: check/wall-ms 100 -> 70, commit the lock (bench)\n|; perf: 0 regressed, 0 unseeded, 1 improved, 0 resolved, 0 held",
        )),
    },
    Case {
        name: CaseName("an_improvement_in_a_run_that_leaves_the_lock_says_so"),
        word: Word::Succeeds,
        input: || Question::Perf(Outcomes::ImprovedWithheld),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "improved: check/wall-ms 100 -> 70, the lock is left as it is: a plain `asymptote check` rewrites the best (bench)\n|; perf: 0 regressed, 0 unseeded, 1 improved, 0 resolved, 0 held",
        )),
    },
    Case {
        name: CaseName("a_record_no_longer_reported_is_resolved"),
        word: Word::Succeeds,
        input: || Question::Perf(Outcomes::Resolved),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "resolved: check/wall-ms is no longer reported by bench; `asymptote down` drops its record\n|; perf: 0 regressed, 0 unseeded, 0 improved, 1 resolved, 0 held",
        )),
    },
    Case {
        name: CaseName("medians_within_their_record_print_only_the_counts"),
        word: Word::Succeeds,
        input: || Question::Perf(Outcomes::Held),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "|; perf: 0 regressed, 0 unseeded, 0 improved, 0 resolved, 2 held",
        )),
    },
    Case {
        name: CaseName("a_run_that_measured_nothing_prints_nothing_of_perf"),
        word: Word::Succeeds,
        input: || Question::Perf(Outcomes::Nothing),
        expect: Expect::Value(std::borrow::Cow::Borrowed("")),
    },
    Case {
        name: CaseName("a_workspace_ends_as_the_worst_of_its_products"),
        word: Word::Succeeds,
        input: || {
            Question::Printed(
                Layout::Workspace,
                vec![
                    ("crates/a", Verdict::Changed),
                    ("crates/b", Verdict::Undetermined),
                ],
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "crates/a: 0 new, 0 resolved\ncrates/b: 0 new, 0 resolved\n|Error",
        )),
    },
    Case {
        name: CaseName(
            "a_workspace_of_clean_products_succeeds_with_each_line_prefixed_by_its_path",
        ),
        word: Word::Succeeds,
        input: || {
            Question::Printed(
                Layout::Workspace,
                vec![(".", Verdict::Pass), ("crates/a", Verdict::Pass)],
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            ".: 0 new, 0 resolved\ncrates/a: 0 new, 0 resolved\n|Success",
        )),
    },
    Case {
        name: CaseName("a_workspace_with_something_new_fails"),
        word: Word::Succeeds,
        input: || {
            Question::Printed(
                Layout::Workspace,
                vec![("crates/a", Verdict::Changed), ("crates/b", Verdict::Pass)],
            )
        },
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "crates/a: 0 new, 0 resolved\ncrates/b: 0 new, 0 resolved\n|Failure",
        )),
    },
    Case {
        name: CaseName("one_product_prints_as_it_always_did"),
        word: Word::Succeeds,
        input: || Question::Printed(Layout::Product, vec![(".", Verdict::Pass)]),
        expect: Expect::Value(std::borrow::Cow::Borrowed("0 new, 0 resolved\n|Success")),
    },
    Case {
        name: CaseName("a_blank_since_is_refused"),
        word: Word::Refuses,
        input: || Question::Line(vec!["--since", " "]),
        expect: Expect::Refusal(|said| said.contains("since")),
    },
    Case {
        name: CaseName("a_fix_under_a_dry_run_is_refused_because_a_fix_command_writes"),
        word: Word::Refuses,
        input: || Question::Previewed(vec!["--fix"]),
        expect: Expect::Refusal(|said| said.contains("--fix") && said.contains("--dry-run")),
    },
    Case {
        name: CaseName("a_check_under_a_dry_run_is_a_request"),
        word: Word::Succeeds,
        input: || Question::Previewed(Vec::new()),
        expect: Expect::Value(std::borrow::Cow::Borrowed("lock All")),
    },
    Case {
        name: CaseName("one_product_that_cannot_answer_ends_the_command_as_an_error"),
        word: Word::Refuses,
        input: || Question::Unanswered,
        expect: Expect::Refusal(|said| said == "Error"),
    },
    Case {
        name: CaseName("a_hook_asked_of_a_manifest_that_maps_none_says_how_to_map_it"),
        word: Word::Succeeds,
        input: || Question::Refused(""),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "`--hook pre-push` is not mapped in the root asymptote.toml: asymptote.toml maps no git hooks; add [hooks] pre-push = [\"<group>\"]",
        )),
    },
    Case {
        name: CaseName("a_hook_asked_of_a_manifest_that_maps_others_lists_them"),
        word: Word::Succeeds,
        input: || Question::Refused("pre-commit"),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "`--hook pre-push` is not mapped in the root asymptote.toml: it maps pre-commit",
        )),
    },
    Case {
        name: CaseName("a_failed_fix_is_in_the_json_as_how_it_ended"),
        word: Word::Succeeds,
        input: || Question::FixJson(true),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            r#"[{"error":{"code":2,"ended":"exited","said":"cannot fix\nthe second line"},"tool":"lint"}]"#,
        )),
    },
    Case {
        name: CaseName("a_tool_that_failed_is_said_on_stderr_whatever_the_format"),
        word: Word::Succeeds,
        input: || Question::Said(Layout::Product),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "lint: fix exited 2: cannot fix\nbench: failed: exited 4: TOOL-ERR",
        )),
    },
    Case {
        name: CaseName("a_workspace_says_a_failed_tool_under_its_products_path"),
        word: Word::Succeeds,
        input: || Question::Said(Layout::Workspace),
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            "crates/a: lint: fix exited 2: cannot fix\ncrates/a: bench: failed: exited 4: TOOL-ERR",
        )),
    },
    Case {
        name: CaseName("groups_the_tools_declare_are_known"),
        word: Word::Succeeds,
        input: || Question::Known(vec!["--groups", "perf,lint"]),
        expect: Expect::Value(std::borrow::Cow::Borrowed("known")),
    },
    Case {
        name: CaseName("no_groups_runs_every_tool_and_is_known"),
        word: Word::Succeeds,
        input: || Question::Known(Vec::new()),
        expect: Expect::Value(std::borrow::Cow::Borrowed("known")),
    },
    Case {
        name: CaseName("a_group_no_tool_declares_is_refused_naming_those_they_do"),
        word: Word::Refuses,
        input: || Question::Known(vec!["--groups", "lint,nosuch"]),
        expect: Expect::Refusal(|said| {
            said == "`--groups nosuch` names a group no tool declares: they declare lint, perf"
        }),
    },
    Case {
        name: CaseName("a_fix_that_succeeds_leaves_no_trace_in_the_json"),
        word: Word::Succeeds,
        input: || Question::FixJson(false),
        expect: Expect::Value(std::borrow::Cow::Borrowed("absent")),
    },
];

/// Reads a command line into this command's arguments.
#[derive(clap::Parser)]
struct Line {
    #[command(flatten)]
    arguments: CheckArguments,
}

fn record() -> PerfRecord {
    PerfRecord {
        key: PerfKey {
            tool: ToolId("bench".into()),
            benchmark: BenchmarkName("check".into()),
            measure: MeasureName("wall-ms".into()),
        },
        best: Number(100.0),
        reasons: vec![Reason("seeded".into())],
    }
}

/// The perf lines of `outcome`, then `|` and its counts when it has any to say.
fn perf(outcomes: Outcomes) -> String {
    let mut outcome = PerfOutcome::default();
    match outcomes {
        Outcomes::Regressed => outcome.regressed.push(Regression {
            record: record(),
            value: Number(120.0),
        }),
        Outcomes::Unseeded => outcome.unseeded.push(Unseeded {
            key: PerfKey {
                tool: ToolId("bench".into()),
                benchmark: BenchmarkName("check".into()),
                measure: MeasureName("wall-ms".into()),
            },
            value: Number(120.0),
        }),
        Outcomes::Improved => outcome.improved.push(Improvement {
            record: record(),
            value: Number(70.0),
        }),
        Outcomes::ImprovedWithheld => {
            outcome.improved.push(Improvement {
                record: record(),
                value: Number(70.0),
            });
            outcome.rewriting = Rewriting::Withheld;
        }
        Outcomes::Resolved => outcome.resolved.push(record()),
        Outcomes::Held => outcome.held = Count(2),
        Outcomes::Nothing => {}
    }
    let schema = PerfSchema::from(&outcome);
    let lines = Printed(&schema, false).to_string();
    if schema.spoke() {
        format!("{lines}|{}", Printed(&schema, true))
    } else {
        lines
    }
}

/// A perf schema's lines, or its counts.
struct Printed<'a>(&'a PerfSchema, bool);

impl std::fmt::Display for Printed<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.1 {
            self.0.counts(f)
        } else {
            self.0.lines(f)
        }
    }
}

/// A report of one product that ran no tool and ends as `verdict`.
fn report(verdict: Verdict) -> CheckReport {
    CheckReport {
        root: ProductRoot("/p".into()),
        runs: Vec::new(),
        outcome: asymptote_core::domain::judgement::types::Judgement::default(),
        verdict,
        against: Against::Lock,
        perf: PerfOutcome::default(),
        failed_fixes: Vec::new(),
    }
}

/// The JSON lists of a run with a new suppressed violation, a resolved record and an unjudged one.
fn judged() -> Result<String, String> {
    use asymptote_core::domain::judgement::types::{Resolution, Resolved, Stale, StaleReason};
    use asymptote_core::domain::operations::types::{Day, Provenance, Record};
    use asymptote_core::domain::violations::types::{
        Fingerprint, Fingerprinted, Id, Justification, Key, Level, LineNumber, MessageText, Place,
        RepoPath, RuleId, Standing, Suppression, Violation,
    };
    let mut checked = report(Verdict::Changed);
    checked.outcome.new.push(Violation {
        path: RepoPath("src/a.rs".into()),
        place: Place::Line(LineNumber(1)),
        tool: ToolId("lint".into()),
        rule: RuleId("r".into()),
        level: Level::Warning,
        suppression: Suppression::Suppressed(Justification("generated".into())),
        message: MessageText("m".into()),
        fingerprint: Fingerprinted::Known(Fingerprint(255)),
    });
    let record = Record {
        id: Id("x".into()),
        key: Key {
            tool: ToolId("lint".into()),
            rule: RuleId("r".into()),
            path: RepoPath("src/b.rs".into()),
            level: Level::Warning,
            standing: Standing::Reported,
            fingerprint: Fingerprint(255),
        },
        count: Count(2),
        message: MessageText("m".into()),
        since_rev: Provenance::Unknown,
        since: Day("2026-10-28".into()),
        reasons: vec![Reason("why".into())],
    };
    checked.outcome.resolved.push(Resolved {
        record: record.clone(),
        count: Count(1),
        remaining: Count(1),
        resolution: Resolution::NotReported,
    });
    checked.outcome.stale.push(Stale {
        record,
        reason: StaleReason::ToolNotRun,
    });
    let json = serde_json::to_value(super::CheckReportSchema::from(&checked))
        .map_err(|error| error.to_string())?;
    let list = |name: &str| {
        json.get(name)
            .map_or_else(|| "absent".to_owned(), ToString::to_string)
    };
    Ok(format!(
        "{} | {} | {}",
        list("violations"),
        list("suppressed"),
        list("unjudged")
    ))
}

/// The lines said on stderr for a product, at `crates/a` in a workspace, whose fix and tool failed.
fn said(layout: Layout) -> String {
    let mut checked = report(Verdict::Undetermined);
    checked.failed_fixes.push(FailedFix {
        tool: ToolId("lint".into()),
        failure: asymptote_core::domain::judgement::types::Failure::Exited {
            code: Code(2),
            stderr: Tail("cannot fix".into()),
        },
    });
    checked.runs.push(ToolRun {
        run: Run {
            tool: ToolId("bench".into()),
            status: Status::Failed(RunFailure::Exited {
                code: Code(4),
                stderr: Tail("TOOL-ERR".into()),
            }),
            inventory: Inventory::Undeclared,
            reported: std::collections::BTreeSet::new(),
        },
        reuse: Reuse::Ran,
    });
    let path = match layout {
        Layout::Product => ".",
        Layout::Workspace => "crates/a",
    };
    let checked = vec![super::CheckedSchema {
        path: path.to_owned(),
        answer: Ok(checked),
    }];
    super::said(layout, &checked).join("\n")
}

/// The request the arguments after `check` become at an edge that writes as `writing` says, or why the schemas refused them.
fn requested(
    line: Vec<&'static str>,
    writing: asymptote_core::domain::ratchet::types::Writing,
) -> Result<CheckRequest, String> {
    let parsed = <Line as clap::Parser>::try_parse_from(std::iter::once("check").chain(line))
        .map_err(|error| error.to_string())?;
    let edge = crate::edge::Edge {
        start: ProductRoot("/repo".into()),
        here: ProductRoot("/repo".into()),
        store: std::path::PathBuf::from(asymptote_core::domain::operations::constants::CACHE),
        build: asymptote_core::outbound::check_store::types::Build("test".into()),
        format: crate::output::Format::Terminal,
        writing,
    };
    super::request(
        parsed.arguments.since,
        parsed.arguments.fix,
        parsed.arguments.groups,
        &edge,
    )
    .map_err(|error| error.to_string())
}

/// Answers the row's question, or says why the schemas refused it.
fn subject(question: Input) -> Result<Value, Failure> {
    let text = match question {
        Question::Line(line) => {
            let request = requested(line, asymptote_core::domain::ratchet::types::Writing::Write)?;
            let against = match request.against {
                Against::Since(_) => "since",
                Against::Lock => "lock",
            };
            format!("{against} {:?}", request.groups)
        }
        Question::Previewed(line) => {
            let request = requested(
                line,
                asymptote_core::domain::ratchet::types::Writing::DryRun,
            )?;
            format!("lock {:?}", request.groups)
        }
        Question::Perf(outcomes) => perf(outcomes),
        Question::Printed(layout, verdicts) => {
            let products = verdicts
                .into_iter()
                .map(|(path, verdict)| (path.to_owned(), Ok(report(verdict))))
                .collect();
            let (printed, outcome) =
                answered(layout, products, None).map_err(|outcome| format!("{outcome:?}"))?;
            format!("{printed}|{outcome:?}")
        }
        Question::Refused(mapped) => super::CheckSchemaError::Hook {
            given: "pre-push".to_owned(),
            mapped: mapped.to_owned(),
        }
        .to_string(),
        Question::Judged => judged()?,
        Question::FixJson(failed) => {
            let mut checked = report(Verdict::Pass);
            if failed {
                checked.failed_fixes.push(FailedFix {
                    tool: ToolId("lint".into()),
                    failure: asymptote_core::domain::judgement::types::Failure::Exited {
                        code: Code(2),
                        stderr: Tail("cannot fix\nthe second line".into()),
                    },
                });
            }
            let json = serde_json::to_value(super::CheckReportSchema::from(&checked))
                .map_err(|error| error.to_string())?;
            json.get("failed_fixes")
                .map_or_else(|| "absent".to_owned(), ToString::to_string)
        }
        Question::Known(line) => {
            let request = requested(line, asymptote_core::domain::ratchet::types::Writing::Write)?;
            let declared = [
                Group("perf".into()),
                Group("lint".into()),
                Group("perf".into()),
            ];
            super::known(&request, None, &declared).map_err(|error| error.to_string())?;
            "known".to_owned()
        }
        Question::Said(layout) => said(layout),
        Question::Unanswered => {
            let products = vec![(".".to_owned(), Err(CheckError::NoHead))];
            return answered(Layout::Product, products, None)
                .map(|_| Value::default())
                .map_err(|outcome| format!("{outcome:?}"));
        }
    };
    Ok(std::borrow::Cow::Owned(text))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
