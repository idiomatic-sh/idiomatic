//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed. `down` takes no selector, so no conversion of it can be refused.

use asymptote_core::domain::judgement::types::Resolution;
use asymptote_core::domain::manifest::types::Layout;
use asymptote_core::domain::ratchet::types::{Compaction, DropReason, Moved, Writing};
use asymptote_core::inbound::use_cases::down::errors::DownError;
use asymptote_core::inbound::use_cases::down::types::{DownReport, DownRequest};

use crate::outcome::Outcome;
use crate::output::schemas::{
    AnswerSchema, MemberSchema, ViolationSchema, WorkspaceSchema, record,
};

/// The status of a record `down` drains.
const RESOLVED: &str = asymptote_core::domain::judgement::constants::RESOLVED;
/// The status of a new in-source ignore `down` records.
const ACCEPTED: &str = asymptote_core::domain::judgement::constants::ACCEPTED;

/// The service's request for the arguments.
pub(super) fn request(edge: &crate::edge::Edge) -> DownRequest {
    DownRequest {
        start: edge.start.clone(),
        writing: edge.writing,
    }
}

/// One record dropped or lowered: the record, resolved (`count` of its violations no longer occur, `remaining` still
/// do: 0 for one dropped), why, and the reasons it was accepted with.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct DrainedSchema {
    /// The record.
    #[serde(flatten)]
    pub record: ViolationSchema,
    /// Why: `fixed`, `retired`, `renamed` or `lowered`.
    pub why: String,
    /// The reasons it was accepted with.
    pub reasons: Vec<String>,
}

/// A perf record's key.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct PerfKeySchema {
    /// The tool.
    pub tool: String,
    /// The benchmark.
    pub benchmark: String,
    /// The measure.
    pub measure: String,
}

/// What `down` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct DownReportSchema {
    /// The records drained.
    pub drained: Vec<DrainedSchema>,
    /// The new in-source ignores recorded as suppressed.
    pub recorded: Vec<ViolationSchema>,
    /// The perf records dropped: their tool ran and no longer reports them.
    pub perf: Vec<PerfKeySchema>,
    /// Whether the lock was written.
    pub written: bool,
    /// Whether it was compacted.
    pub compacted: bool,
}

impl std::fmt::Display for DownReportSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = if self.written {
            "drained"
        } else {
            "would drain"
        };
        for drained in &self.drained {
            // A suppressed record carries no reason: its reason is the in-source ignore beside its code.
            let accepted = if drained.reasons.is_empty() {
                "ignored in the source".to_owned()
            } else {
                format!("accepted for: {}", drained.reasons.join("; "))
            };
            let row = &drained.record;
            writeln!(
                f,
                "{verb} {}: {}/{} ({}, {} left; {accepted})",
                row.path,
                row.tool,
                row.rule,
                drained.why,
                row.remaining.unwrap_or_default(),
            )?;
        }
        let recording = if self.written {
            "recorded"
        } else {
            "would record"
        };
        for record in &self.recorded {
            writeln!(
                f,
                "{recording} {}: {}/{} ({}, ignored in the source)",
                record.path,
                record.tool,
                record.rule,
                record.count.unwrap_or_default()
            )?;
        }
        for dropped in &self.perf {
            writeln!(
                f,
                "{verb} perf {}/{}/{}",
                dropped.tool, dropped.benchmark, dropped.measure
            )?;
        }
        write!(f, "{} {verb}", self.drained.len())?;
        if !self.recorded.is_empty() {
            write!(f, "; {} {recording}", self.recorded.len())?;
        }
        if !self.perf.is_empty() {
            write!(f, "; {} perf {verb}", self.perf.len())?;
        }
        if self.compacted {
            write!(f, "; the lock was compacted")?;
        }
        writeln!(f)
    }
}

/// The answer printed for the service's report.
pub(super) fn answer(report: &DownReport) -> DownReportSchema {
    report.into()
}

impl From<&DownReport> for DownReportSchema {
    fn from(report: &DownReport) -> Self {
        drained(report)
    }
}

/// The answer printed for the service's report: each record drained or lowered, and the counts.
fn drained(report: &DownReport) -> DownReportSchema {
    let reasons = |record: &asymptote_core::domain::operations::types::Record| {
        record
            .reasons
            .iter()
            .map(|reason| reason.0.to_string())
            .collect()
    };
    let mut drained: Vec<DrainedSchema> = report
        .dropped
        .iter()
        .map(|dropped| DrainedSchema {
            record: ViolationSchema {
                remaining: Some(0),
                ..record(&dropped.record, RESOLVED)
            },
            why: match (dropped.reason, &dropped.moved) {
                (DropReason::Resolved(Resolution::NotReported), _) => "fixed".to_owned(),
                (DropReason::Resolved(Resolution::RuleRetired), _) => "retired".to_owned(),
                (DropReason::RenamedWithoutFinding, Moved::To(path)) => {
                    format!("renamed to {}", path.0)
                }
                (DropReason::RenamedWithoutFinding, Moved::Stayed) => "renamed".to_owned(),
            },
            reasons: reasons(&dropped.record),
        })
        .collect();
    drained.extend(report.lowered.iter().map(|lowered| DrainedSchema {
        record: ViolationSchema {
            count: Some(lowered.from.0.saturating_sub(lowered.record.count.0)),
            remaining: Some(lowered.record.count.0),
            ..record(&lowered.record, RESOLVED)
        },
        why: "lowered".to_owned(),
        reasons: reasons(&lowered.record),
    }));
    DownReportSchema {
        drained,
        recorded: report
            .recorded
            .iter()
            .map(|written| record(written, ACCEPTED))
            .collect(),
        perf: report
            .perf
            .iter()
            .map(|record| PerfKeySchema {
                tool: record.key.tool.0.to_string(),
                benchmark: record.key.benchmark.0.to_string(),
                measure: record.key.measure.0.to_string(),
            })
            .collect(),
        written: report.writing == Writing::Write,
        compacted: report.compaction == Compaction::Snapshot,
    }
}

/// The answer for what each product answered, and the outcome it ends the command as: an error when a product could
/// not answer, which is said on stderr, and a success otherwise.
///
/// # Errors
///
/// The outcome the command ends as, when one product alone was reached and could not answer.
pub(super) fn answered(
    layout: Layout,
    products: Vec<(String, Result<DownReport, DownError>)>,
) -> Result<(AnswerSchema<DownReportSchema>, Outcome), Outcome> {
    if layout == Layout::Product {
        return match products.into_iter().next() {
            Some((_, Ok(report))) => Ok((
                AnswerSchema::Product(Box::new(answer(&report))),
                Outcome::Success,
            )),
            Some((_, Err(error))) => Err(crate::output::fail(&error, Outcome::Error)),
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
    for (path, answered) in products {
        match answered {
            Ok(report) => members.push(MemberSchema {
                path,
                report: answer(&report),
            }),
            Err(error) => outcome = crate::output::fail(&error, Outcome::Error),
        }
    }
    Ok((
        AnswerSchema::Workspace(WorkspaceSchema { members }),
        outcome,
    ))
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

impl crate::output::Encoded for DownReportSchema {
    fn json(
        &self,
        to: &mut dyn std::io::Write,
        spacing: crate::output::Spacing,
    ) -> std::io::Result<()> {
        match spacing {
            crate::output::Spacing::Compact => serde_json::to_writer(to, self),
            crate::output::Spacing::Indented => serde_json::to_writer_pretty(to, self),
        }
        .map_err(std::io::Error::from)
    }
}
