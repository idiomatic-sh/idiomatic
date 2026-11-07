// ignore: idiomatic.arch/file-length - the command spec places one schemas.rs per command, the only file of a command that holds wire shapes and validation, and gives no second one: check's answer is three schema families (the findings, the perf medians, the products of a workspace), and splitting them needs a file role the spec does not have.
//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed: one compiler-style line per violation, `path:line:column: tool/rule: message`,
//! sorted by path and line, then the summary; JSON of the same fields; or the SARIF log the service wrote. Each
//! refusal of a conversion is a `<Name>SchemaError`.

use asymptote_core::domain::judgement::constants::ACCEPTED;

/// The status of a record some of whose violations no longer occur.
const RESOLVED: &str = asymptote_core::domain::judgement::constants::RESOLVED;
use asymptote_core::domain::judgement::types::{
    Failure, Judgement as Judged, StaleReason, Status, Verdict,
};
use asymptote_core::domain::manifest::service::{groups, hooked, mapped};
use asymptote_core::domain::manifest::types::{Group, Groups, HookWord, Layout};
use asymptote_core::domain::operations::types::Revision;
use asymptote_core::domain::operations::types::{BenchmarkName, MeasureName, PerfKey};
use asymptote_core::domain::perf::types::{PerfOutcome, Rewriting};
use asymptote_core::domain::violations::types::{Justification, Standing, Suppression, ToolId};
use asymptote_core::inbound::use_cases::check::errors::CheckError;
use asymptote_core::inbound::use_cases::check::types::{
    Against, CheckReport, CheckRequest, Fixing, Reuse,
};
use asymptote_core::outbound::manifest::types::HookTable;

use crate::outcome::Outcome;
use crate::output::Format;
use crate::output::schemas::{
    AnswerSchema, MemberSchema, ViolationSchema, WorkspaceSchema, place, prefixed, record,
    violation,
};

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum CheckSchemaError {
    /// `--since` names nothing.
    #[error("`--since` names no revision")]
    EmptySince,
    /// `--fix` under `--dry-run`: a fix command rewrites files, which a dry run writes none of.
    #[error("`--fix` cannot be previewed with `--dry-run`: a tool's fix command rewrites files")]
    FixDryRun,
    /// A `--hook` names a hook the root's `[hooks]` does not map.
    #[error("`--hook {given}` is not mapped in the root asymptote.toml: {}", maps(.given, .mapped))]
    Hook {
        /// What was given.
        given: String,
        /// The hooks it maps.
        mapped: String,
    },
    /// A SARIF log is one product's: a workspace has no one log.
    #[error("`--format sarif` is one log of one product: a workspace root runs several")]
    WorkspaceSarif,
    /// A `--groups` names a group no tool of the products run declares.
    #[error("`--groups {group}` names a group no tool declares: {}", declares(.declared))]
    UnknownGroup {
        /// The group named.
        group: String,
        /// The groups the tools declare, comma separated.
        declared: String,
    },
}

/// The groups the tools declare, said for a refused `--groups`.
fn declares(declared: &str) -> String {
    if declared.is_empty() {
        "no tool declares one".to_owned()
    } else {
        format!("they declare {declared}")
    }
}

/// What the root `[hooks]` maps, said for a refused `--hook given`: the hooks it maps, or that it maps none and how to map one.
fn maps(given: &str, mapped: &str) -> String {
    if mapped.is_empty() {
        format!("asymptote.toml maps no git hooks; add [hooks] {given} = [\"<group>\"]")
    } else {
        format!("it maps {mapped}")
    }
}

/// What the user does next about a command line the schemas refuse.
impl crate::outcome::remedy::Remedy for CheckSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::EmptySince => crate::outcome::remedy::Next::Fix(
                "name the revision to compare with: `--since origin/main`".to_owned(),
            ),
            Self::FixDryRun => crate::outcome::remedy::Next::Fix(
                "run `asymptote check --fix` to apply the fixes, or `asymptote check --dry-run` to preview the check".to_owned(),
            ),
            Self::Hook { given, mapped } if mapped.is_empty() => crate::outcome::remedy::Next::Fix(format!(
                "map it in the root asymptote.toml: `[hooks] {given} = [\"<group>\"]`"
            )),
            Self::Hook { mapped, .. } => crate::outcome::remedy::Next::Fix(format!(
                "name a hook the root asymptote.toml maps ({mapped}), or map it in `[hooks]`"
            )),
            Self::WorkspaceSarif => crate::outcome::remedy::Next::Fix(
                "run inside a member for its SARIF log, or print the workspace with `--format json`".to_owned(),
            ),
            Self::UnknownGroup { declared, .. } => crate::outcome::remedy::Next::Fix(format!(
                "name a group a tool's `group` declares ({}), or none to run every tool",
                declares(declared)
            )),
        }
    }
}

/// The arguments as the command line gives them, offered to the conversion.
struct CheckArgumentsSchema<'a> {
    /// Judge against what REV's branch accepted, so only what this branch introduced since REV is new; the lock is
    /// never written.
    since: Option<String>,
    /// Run each tool's fix command first, saying on stderr each one that fails; the lock is never written.
    fix: bool,
    /// Run only the tools of these groups (the labels the tools carry), comma separated; with none, every tool runs.
    groups: Vec<String>,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
}

impl TryFrom<CheckArgumentsSchema<'_>> for CheckRequest {
    type Error = CheckSchemaError;

    fn try_from(schema: CheckArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        let words: Vec<Group> = schema
            .groups
            .iter()
            .map(|word| Group(word.as_str().into()))
            .collect();
        if schema.fix && edge.writing == asymptote_core::domain::ratchet::types::Writing::DryRun {
            return Err(CheckSchemaError::FixDryRun);
        }
        let against = match schema.since {
            None => Against::Lock,
            Some(since) if since.trim().is_empty() => return Err(CheckSchemaError::EmptySince),
            Some(since) => Against::Since(Revision(since.into())),
        };
        Ok(Self {
            start: edge.start.clone(),
            against,
            fixing: if schema.fix {
                Fixing::Fix
            } else {
                Fixing::Report
            },
            groups: groups(&words),
        })
    }
}

/// `request` run for the groups the hook `word` is mapped to in `table`; as it is when no hook is asked for.
///
/// # Errors
///
/// `Hook` when the table does not map it.
pub(super) fn hooked_request(
    request: CheckRequest,
    word: Option<&str>,
    table: Option<&HookTable>,
) -> Result<CheckRequest, CheckSchemaError> {
    let (Some(word), Some(table)) = (word, table) else {
        return Ok(request);
    };
    let selected =
        hooked(&table.0, &HookWord(word.into())).ok_or_else(|| CheckSchemaError::Hook {
            given: word.to_owned(),
            mapped: mapped(&table.0).0.to_string(),
        })?;
    Ok(CheckRequest {
        groups: selected,
        ..request
    })
}

/// Refuses a `--groups` that names a group none of `declared` is: a typo would otherwise run nothing and pass. A
/// `--hook`'s groups were held to the whole workspace's by the root's table, so they are not held again here.
///
/// # Errors
///
/// `UnknownGroup` naming the first such group.
pub(super) fn known(
    request: &CheckRequest,
    hook: Option<&str>,
    declared: &[Group],
) -> Result<(), CheckSchemaError> {
    let (None, Groups::Only(named)) = (hook, &request.groups) else {
        return Ok(());
    };
    match named.iter().find(|group| !declared.contains(group)) {
        None => Ok(()),
        Some(unknown) => {
            let mut listed: Vec<&str> = declared.iter().map(|group| &*group.0).collect();
            listed.sort_unstable();
            listed.dedup();
            Err(CheckSchemaError::UnknownGroup {
                group: unknown.0.to_string(),
                declared: listed.join(", "),
            })
        }
    }
}

/// Refuses a SARIF log of a workspace.
///
/// # Errors
///
/// `WorkspaceSarif` when `format` is SARIF and the products are several.
pub(super) fn layout(layout: Layout, format: Format) -> Result<(), CheckSchemaError> {
    if layout == Layout::Workspace && format == Format::Sarif {
        return Err(CheckSchemaError::WorkspaceSarif);
    }
    Ok(())
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `EmptySince` for a `--since` that names nothing; `FixDryRun` for a `--fix` under `--dry-run`.
pub(super) fn request(
    since: Option<String>,
    fix: bool,
    groups: Vec<String>,
    edge: &crate::edge::Edge,
) -> Result<CheckRequest, CheckSchemaError> {
    CheckArgumentsSchema {
        since,
        fix,
        groups,
        edge,
    }
    .try_into()
}

/// A violation an in-source ignore suppresses, as JSON reads it: the violation, and the ignore's reason.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) struct SuppressedSchema {
    /// The violation.
    #[serde(flatten)]
    // ignore: idiomatic.arch/pub-item-unreferenced-elsewhere - a field of the printed wire shape: serde reads it, and the shape keeps one visibility for every field (clippy::partial_pub_fields)
    pub finding: ViolationSchema,
    /// Why the ignore suppresses it, as it says; empty where it says nothing.
    pub reason: String,
}

/// A record that cannot be judged, as JSON reads it: the record, and why.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(super) struct UnjudgedSchema {
    /// The record the lock accepts.
    #[serde(flatten)]
    pub record: ViolationSchema,
    /// Why: `tool-not-run`, `tool-failed` or `rule-not-evaluated`.
    pub why: String,
}

/// How a run ended when it failed, as JSON reads it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct FailureSchema {
    /// `exited`, `signalled`, `timed-out`, `unstartable` or `unreadable`.
    pub ended: String,
    /// The exit code, for `exited`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub code: Option<i32>,
    /// The timeout, for `timed-out`.
    #[serde(skip_serializing_if = "Option::is_none")]
    // ignore: idiomatic.arch/pub-item-unreferenced-elsewhere - a field of the printed wire shape: serde reads it, and the shape keeps one visibility for every field (clippy::partial_pub_fields)
    pub after_seconds: Option<u64>,
    /// What it last said, or why it could not start or be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub said: Option<String>,
}

/// Full failure streams, or an explicit lack of retained output for older records.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(tag = "availability", rename_all = "kebab-case")]
pub(super) enum FailureOutputSchema {
    /// No capture was retained; this is not an observed empty stream.
    Unavailable,
    /// Exact streams from a failed completion.
    Captured {
        /// Complete standard output.
        stdout: String,
        /// Complete standard error.
        stderr: String,
        /// Command and input fingerprint; absent when inputs could not be stamped.
        inputs_digest: Option<String>,
    },
}

impl From<&asymptote_core::domain::reports::types::FailureOutput> for FailureOutputSchema {
    fn from(output: &asymptote_core::domain::reports::types::FailureOutput) -> Self {
        use asymptote_core::domain::reports::types::{FailureOutput, Stamped};
        match output {
            FailureOutput::Unavailable => Self::Unavailable,
            FailureOutput::Captured {
                stdout,
                stderr,
                inputs,
            } => Self::Captured {
                stdout: stdout.0.to_string(),
                stderr: stderr.0.to_string(),
                inputs_digest: match inputs {
                    Stamped::Digested(digest) => Some(digest.0.to_string()),
                    Stamped::Unstamped => None,
                },
            },
        }
    }
}

/// One tool's run as JSON reads it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct ToolSchema {
    /// Full streams for failed runs, absent for successful runs.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub diagnostics: Option<FailureOutputSchema>,
    /// The tool.
    pub id: String,
    /// Whether its last run was reused, its inputs unmoved.
    pub reused: bool,
    /// How it failed; absent when it completed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<FailureSchema>,
}

/// A fix command that did not succeed, as JSON reads it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct FailedFixSchema {
    /// The tool whose fix it was.
    pub tool: String,
    /// How it failed.
    pub error: FailureSchema,
}

/// The counts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct SummarySchema {
    /// New violations: introduced since `--since`'s revision, when it is given.
    pub new: usize,
    /// Records some of whose violations no longer occur.
    pub resolved: usize,
    /// Violations the baseline accepts.
    pub accepted: usize,
    /// Records that cannot be judged.
    pub unjudged: usize,
}

/// What `check` prints.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(super) struct CheckReportSchema {
    /// Each tool's run.
    pub tools: Vec<ToolSchema>,
    /// Each violation, new, resolved and accepted, sorted by path and line: an in-source ignore's are in `suppressed`.
    pub violations: Vec<ViolationSchema>,
    /// Each violation an in-source ignore suppresses, new, resolved and accepted, with the ignore's reason.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub suppressed: Vec<SuppressedSchema>,
    /// Each record that cannot be judged, and why.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub unjudged: Vec<UnjudgedSchema>,
    /// The revision `--since` named.
    pub since: Option<String>,
    /// The counts.
    pub summary: SummarySchema,
    /// What the tools measured, against the lock's perf records; absent from a run that measured nothing, so the answer of a
    /// product with no perf tool is what it was before there were any.
    #[serde(skip_serializing_if = "PerfSchema::silent")]
    pub perf: PerfSchema,
    /// The fix commands of `--fix` that did not succeed; absent when none failed, so an answer without one is what it
    /// was before.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub failed_fixes: Vec<FailedFixSchema>,
    /// The SARIF log, under `--format sarif`.
    #[serde(skip)]
    pub sarif: Option<String>,
}

/// Why a tool failed, said for the user.
fn failure(failure: &Failure) -> String {
    let said = |stderr: &str| {
        if stderr.is_empty() {
            String::new()
        } else {
            format!(": {stderr}")
        }
    };
    match failure {
        Failure::Exited { code, stderr } => format!("exited {}{}", code.0, said(&stderr.0)),
        Failure::Signalled { stderr } => format!("a signal ended it{}", said(&stderr.0)),
        Failure::TimedOut { after } => format!("ran past its timeout of {} s", after.0),
        Failure::Unstartable(problem) => format!("could not start: {}", problem.0),
        Failure::Unreadable { problem, stderr } => format!("{}{}", problem.0, said(&stderr.0)),
    }
}

/// How a fix failed, said for the user: the first line it wrote to stderr stands for what it said.
fn fix_failure(failure: &Failure) -> String {
    let said = |stderr: &str| {
        stderr
            .lines()
            .next()
            .map_or_else(String::new, |first| format!(": {first}"))
    };
    match failure {
        Failure::Exited { code, stderr } => format!("fix exited {}{}", code.0, said(&stderr.0)),
        Failure::Signalled { stderr } => format!("fix was ended by a signal{}", said(&stderr.0)),
        Failure::TimedOut { after } => format!("fix timed out after {} s", after.0),
        Failure::Unstartable(problem) => format!("fix could not start: {}", problem.0),
        Failure::Unreadable { problem, .. } => format!("fix could not be read: {}", problem.0),
    }
}

/// How a run ended, as JSON reads it.
fn failure_schema(failure: &Failure) -> FailureSchema {
    let said = |text: &str| (!text.is_empty()).then(|| text.to_owned());
    let (ended, code, after_seconds, words) = match failure {
        Failure::Exited { code, stderr } => ("exited", Some(code.0), None, said(&stderr.0)),
        Failure::Signalled { stderr } => ("signalled", None, None, said(&stderr.0)),
        Failure::TimedOut { after } => ("timed-out", None, Some(after.0), None),
        Failure::Unstartable(problem) => ("unstartable", None, None, said(&problem.0)),
        Failure::Unreadable { problem, .. } => ("unreadable", None, None, said(&problem.0)),
    };
    FailureSchema {
        ended: ended.to_owned(),
        code,
        after_seconds,
        said: words,
    }
}

/// Every violation the judgement names, new, resolved and accepted, as JSON reads them: those no ignore suppresses,
/// then those an in-source ignore does, with its reason.
fn violations(outcome: &Judged) -> (Vec<ViolationSchema>, Vec<SuppressedSchema>) {
    let mut all: Vec<(ViolationSchema, &Suppression)> = outcome
        .new
        .iter()
        .map(|found| (violation(found, "new"), &found.suppression))
        .chain(
            outcome
                .accepted
                .iter()
                .map(|found| (violation(found, ACCEPTED), &found.suppression)),
        )
        .collect();
    // The lock keeps no reason for a suppressed record: the ignore beside the code that was fixed had it.
    let unsaid = Suppression::Suppressed(Justification::default());
    for resolved in &outcome.resolved {
        let key = &resolved.record.key;
        let schema = ViolationSchema {
            count: Some(resolved.count.0),
            remaining: Some(resolved.remaining.0),
            ..record(&resolved.record, RESOLVED)
        };
        all.push((
            schema,
            match key.standing {
                Standing::Reported => &Suppression::Unsuppressed,
                Standing::Suppressed => &unsaid,
            },
        ));
    }
    all.sort_by(|left, right| {
        (&left.0.path, left.0.line, left.0.column).cmp(&(
            &right.0.path,
            right.0.line,
            right.0.column,
        ))
    });
    let mut reported = Vec::new();
    let mut suppressed = Vec::new();
    for (finding, suppression) in all {
        match suppression {
            Suppression::Unsuppressed => reported.push(finding),
            Suppression::Suppressed(why) => suppressed.push(SuppressedSchema {
                finding,
                reason: why.0.to_string(),
            }),
        }
    }
    (reported, suppressed)
}

/// Every record the judgement cannot judge, as JSON reads them.
fn unjudged(outcome: &Judged) -> Vec<UnjudgedSchema> {
    outcome
        .stale
        .iter()
        .map(|stale| UnjudgedSchema {
            record: record(&stale.record, ACCEPTED),
            why: match stale.reason {
                StaleReason::ToolNotRun => "tool-not-run",
                StaleReason::ToolFailed => "tool-failed",
                StaleReason::RuleNotEvaluated => "rule-not-evaluated",
            }
            .to_owned(),
        })
        .collect()
}

impl std::fmt::Display for CheckReportSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // A SARIF log was written only because `--format sarif` asked for it: it is the whole answer then.
        if let Some(sarif) = &self.sarif {
            return f.write_str(sarif);
        }
        // A tool that failed is said on stderr, in every format (`said`), and not again here.
        let ignored = self.suppressed.iter().map(|each| (&each.finding, true));
        let shown = self
            .violations
            .iter()
            .map(|found| (found, false))
            .chain(ignored);
        for (found, suppressed) in shown.filter(|(found, _)| found.status != ACCEPTED) {
            let place = place(found);
            let note = match (found.status.as_str(), suppressed) {
                (RESOLVED, _) => " (resolved)",
                (_, true) => " (an in-source ignore)",
                _ => "",
            };
            writeln!(
                f,
                "{place}: {}/{}: {}{note}",
                found.tool, found.rule, found.message
            )?;
        }
        self.perf.lines(f)?;
        let summary = &self.summary;
        match &self.since {
            Some(since) => write!(
                f,
                "{} introduced since {since}, {} resolved",
                summary.new, summary.resolved
            )?,
            None => write!(f, "{} new, {} resolved", summary.new, summary.resolved)?,
        }
        if summary.unjudged > 0 {
            write!(
                f,
                ", {} unjudged: a tool did not run, or a rule was not evaluated",
                summary.unjudged
            )?;
        }
        if self.perf.spoke() {
            self.perf.counts(f)?;
        }
        writeln!(f)
    }
}

/// The answer printed for the service's report, and the outcome it ends the command as: a failure when something is
/// new or resolved, an error when a tool failed.
pub(super) fn answer(report: &CheckReport) -> (CheckReportSchema, Outcome) {
    let outcome = match report.verdict {
        Verdict::Pass => Outcome::Success,
        Verdict::Changed => Outcome::Failure,
        Verdict::Undetermined => Outcome::Error,
    };
    (report.into(), outcome)
}

impl From<&CheckReport> for CheckReportSchema {
    fn from(report: &CheckReport) -> Self {
        printed(report)
    }
}

/// The answer printed for one product's report: its violations, its tools' runs, the counts and the perf medians.
fn printed(report: &CheckReport) -> CheckReportSchema {
    let tools = report
        .runs
        .iter()
        .map(|run| ToolSchema {
            diagnostics: match run.run.status {
                Status::Failed(_) => Some((&run.failure_output).into()),
                Status::Completed => None,
            },
            id: run.run.tool.0.to_string(),
            reused: run.reuse == Reuse::Reused,
            error: match &run.run.status {
                Status::Completed => None,
                Status::Failed(cause) => Some(failure_schema(cause)),
            },
        })
        .collect();
    let judged = &report.outcome;
    let (reported, suppressed) = violations(judged);
    CheckReportSchema {
        tools,
        violations: reported,
        suppressed,
        unjudged: unjudged(judged),
        since: match &report.against {
            Against::Lock => None,
            Against::Since(revision) => Some(revision.0.to_string()),
        },
        summary: SummarySchema {
            new: judged.new.len(),
            resolved: judged.resolved.len(),
            accepted: judged.accepted.len(),
            unjudged: judged.stale.len(),
        },
        perf: PerfSchema::from(&report.perf),
        failed_fixes: report
            .failed_fixes
            .iter()
            .map(|failed| FailedFixSchema {
                tool: failed.tool.0.to_string(),
                error: failure_schema(&failed.failure),
            })
            .collect(),
        sarif: None,
    }
}

/// How a median stands against the lock.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub(super) enum PerfStatusSchema {
    /// Over the best.
    Regressed,
    /// No record.
    Unseeded,
    /// Under the best.
    Improved,
    /// Its record is no longer reported.
    Resolved,
}

/// One median that is not within its record, or a record that cannot be judged.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(super) struct PerfFindingSchema {
    /// How it stands.
    pub status: PerfStatusSchema,
    /// The tool.
    pub tool: String,
    /// The benchmark.
    pub benchmark: String,
    /// The measure.
    pub measure: String,
    /// The median measured, when the tool reported one.
    pub value: Option<f64>,
    /// The best the lock records, when it records one.
    pub best: Option<f64>,
}

/// The counts.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct PerfSummarySchema {
    /// Medians over their best.
    pub regressed: usize,
    /// Medians with no record.
    pub unseeded: usize,
    /// Medians below their best: the lock's best was rewritten.
    pub improved: usize,
    /// Records no longer reported.
    pub resolved: usize,
    /// Medians within their record.
    pub held: u32,
}

/// What `check` prints of perf.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(super) struct PerfSchema {
    /// The findings.
    pub findings: Vec<PerfFindingSchema>,
    /// The counts.
    pub summary: PerfSummarySchema,
    /// Whether the lock was rewritten for the improvements: it is not printed, only what the improved lines say.
    #[serde(skip)]
    pub rewriting: Rewriting,
}

/// A perf finding of `status` for one measure, with no value or best to show.
fn finding(
    status: PerfStatusSchema,
    tool: &ToolId,
    benchmark: &BenchmarkName,
    measure: &MeasureName,
) -> PerfFindingSchema {
    PerfFindingSchema {
        status,
        tool: tool.0.to_string(),
        benchmark: benchmark.0.to_string(),
        measure: measure.0.to_string(),
        value: None,
        best: None,
    }
}

/// The finding of a record's own measure.
fn recorded(status: PerfStatusSchema, key: &PerfKey) -> PerfFindingSchema {
    finding(status, &key.tool, &key.benchmark, &key.measure)
}

impl From<&PerfOutcome> for PerfSchema {
    fn from(outcome: &PerfOutcome) -> Self {
        let mut findings = Vec::new();
        findings.extend(outcome.regressed.iter().map(|each| {
            let key = &each.record.key;
            PerfFindingSchema {
                value: Some(each.value.0),
                best: Some(each.record.best.0),
                ..finding(
                    PerfStatusSchema::Regressed,
                    &key.tool,
                    &key.benchmark,
                    &key.measure,
                )
            }
        }));
        findings.extend(outcome.unseeded.iter().map(|each| PerfFindingSchema {
            value: Some(each.value.0),
            ..finding(
                PerfStatusSchema::Unseeded,
                &each.key.tool,
                &each.key.benchmark,
                &each.key.measure,
            )
        }));
        findings.extend(outcome.improved.iter().map(|each| {
            let key = &each.record.key;
            PerfFindingSchema {
                value: Some(each.value.0),
                best: Some(each.record.best.0),
                ..finding(
                    PerfStatusSchema::Improved,
                    &key.tool,
                    &key.benchmark,
                    &key.measure,
                )
            }
        }));
        findings.extend(outcome.resolved.iter().map(|each| PerfFindingSchema {
            best: Some(each.best.0),
            ..recorded(PerfStatusSchema::Resolved, &each.key)
        }));
        Self {
            findings,
            summary: PerfSummarySchema {
                regressed: outcome.regressed.len(),
                unseeded: outcome.unseeded.len(),
                improved: outcome.improved.len(),
                resolved: outcome.resolved.len(),
                held: outcome.held.0,
            },
            rewriting: outcome.rewriting,
        }
    }
}

impl PerfSchema {
    /// Whether the run measured anything worth a word in the summary.
    pub(super) fn spoke(&self) -> bool {
        self.summary.held > 0 || !self.findings.is_empty()
    }

    /// Whether perf has nothing to say.
    pub(super) fn silent(&self) -> bool {
        !self.spoke()
    }

    /// Writes one line for each finding.
    pub(super) fn lines(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for each in &self.findings {
            let name = format!("{}/{}", each.benchmark, each.measure);
            let number = |value: Option<f64>| value.unwrap_or_default();
            match each.status {
                PerfStatusSchema::Regressed => writeln!(
                    formatter,
                    "regressed: {name} {} is over the best {} ({})",
                    number(each.value),
                    number(each.best),
                    each.tool
                )?,
                PerfStatusSchema::Unseeded => writeln!(
                    formatter,
                    "unseeded: {name} {}; seed with `asymptote up --reason <why> --tool {}`",
                    number(each.value),
                    each.tool
                )?,
                PerfStatusSchema::Improved => writeln!(
                    formatter,
                    "improved: {name} {} -> {}, {} ({})",
                    number(each.best),
                    number(each.value),
                    match self.rewriting {
                        Rewriting::Written => "commit the lock",
                        Rewriting::Withheld =>
                            "the lock is left as it is: a plain `asymptote check` rewrites the best",
                    },
                    each.tool
                )?,
                PerfStatusSchema::Resolved => writeln!(
                    formatter,
                    "resolved: {name} is no longer reported by {}; `asymptote down` drops its record",
                    each.tool
                )?,
            }
        }
        Ok(())
    }

    /// Writes the counts, after the summary of the findings.
    pub(super) fn counts(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let summary = &self.summary;
        write!(
            formatter,
            "; perf: {} regressed, {} unseeded, {} improved, {} resolved, {} held",
            summary.regressed, summary.unseeded, summary.improved, summary.resolved, summary.held
        )
    }
}

/// What one product answered.
pub(super) struct CheckedSchema {
    /// Its path from the workspace root: `.` for the root itself.
    pub path: String,
    /// The report, or why it could not be made.
    pub answer: Result<CheckReport, CheckError>,
}

/// The one product's report a SARIF log is rendered from, when `--format sarif` asked for one: a workspace has none.
pub(super) fn sarif_report(
    format: Format,
    checked: &[(String, Result<CheckReport, CheckError>)],
) -> Option<&CheckReport> {
    if format != Format::Sarif {
        return None;
    }
    checked.first()?.1.as_ref().ok() // ignore: idiomatic.arch/no-ok-on-result - a product whose check failed has no SARIF log to render: its error is said in the answer (`answered`)
}

/// The answer for what each product answered, and the outcome it ends the command as: for a workspace the worst
/// among the products, an error when one could not answer or a tool failed, a failure when something is new or
/// resolved. A product that could not answer says why on stderr, as does each fix that failed.
///
/// # Errors
///
/// The outcome the command ends as, when one product alone was checked and could not answer.
pub(super) fn answered(
    layout: Layout,
    products: Vec<(String, Result<CheckReport, CheckError>)>,
    sarif: Option<String>,
) -> Result<(AnswerSchema<CheckReportSchema>, Outcome), Outcome> {
    let checked: Vec<CheckedSchema> = products
        .into_iter()
        .map(|(path, answer)| CheckedSchema { path, answer })
        .collect();
    for line in said(layout, &checked) {
        crate::output::said(&line);
    }
    if layout == Layout::Product {
        return match checked.first().map(|each| &each.answer) {
            Some(Ok(report)) => {
                let (schema, outcome) = answer(report);
                Ok((
                    AnswerSchema::Product(Box::new(CheckReportSchema { sarif, ..schema })),
                    outcome,
                ))
            }
            Some(Err(error)) => Err(crate::output::fail(error, Outcome::Error)),
            None => Ok((
                AnswerSchema::Workspace(WorkspaceSchema {
                    members: Vec::new(),
                }),
                Outcome::Success,
            )),
        };
    }
    let mut outcome = Outcome::Success;
    let mut members = Vec::new();
    for each in &checked {
        match &each.answer {
            Ok(report) => {
                let (schema, ended) = answer(report);
                outcome = worst(outcome, ended);
                members.push(MemberSchema {
                    path: each.path.clone(),
                    report: schema,
                });
            }
            Err(error) => {
                outcome = worst(outcome, crate::output::fail(error, Outcome::Error));
            }
        }
    }
    Ok((
        AnswerSchema::Workspace(WorkspaceSchema { members }),
        outcome,
    ))
}

/// The lines said on stderr in every format, `--quiet` included, beside the answer that carries them too: each fix
/// that failed and each tool that failed, an error the exit code alone would leave unexplained. A workspace prefixes
/// each with its product's path.
pub(super) fn said(layout: Layout, checked: &[CheckedSchema]) -> Vec<String> {
    let mut lines = Vec::new();
    for each in checked {
        let Ok(report) = &each.answer else {
            continue;
        };
        let fixes = report
            .failed_fixes
            .iter()
            .map(|failed| format!("{}: {}", failed.tool.0, fix_failure(&failed.failure)));
        let tools = report.runs.iter().filter_map(|ran| match &ran.run.status {
            Status::Failed(cause) => {
                Some(format!("{}: failed: {}", ran.run.tool.0, failure(cause)))
            }
            Status::Completed => None,
        });
        lines.extend(fixes.chain(tools).map(|line| match layout {
            Layout::Product => line,
            Layout::Workspace => prefixed(&each.path, &line),
        }));
    }
    lines
}

/// The worse of two outcomes: an error, then a failure, then a success.
fn worst(left: Outcome, right: Outcome) -> Outcome {
    if left.code() >= right.code() {
        left
    } else {
        right
    }
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
