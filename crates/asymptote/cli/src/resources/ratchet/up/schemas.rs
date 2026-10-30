//! The shape of the data this command does not control: its arguments become the service's request, each `--at` read
//! as `path:line`, and the service's report becomes the answer printed. Each refusal of a conversion is a
//! `<Name>SchemaError`.

use asymptote_core::domain::manifest::types::Layout;
use asymptote_core::domain::operations::types::Reason;
use asymptote_core::domain::ratchet::types::{Compaction, Writing};
use asymptote_core::domain::selection::types::{At, PathPattern, Selectors};
use asymptote_core::domain::violations::service::normalize;
use asymptote_core::domain::violations::types::{LineNumber, RepoPath, RuleId, ToolId};
use asymptote_core::inbound::use_cases::up::errors::UpError;
use asymptote_core::inbound::use_cases::up::types::{UpReport, UpRequest};

use crate::outcome::Outcome;
use crate::output::schemas::{
    AnswerSchema, MemberSchema, ViolationSchema, WorkspaceSchema, place, record, violation,
};

/// The status of a record `up` writes.
const ACCEPTED: &str = asymptote_core::domain::judgement::constants::ACCEPTED;
/// The status of a violation `up` leaves, which still fails `check`.
const NEW: &str = "new";

/// Why the arguments were refused.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub(crate) enum UpSchemaError {
    /// The reason is blank.
    #[error("`--reason` says nothing: say why these findings are accepted")]
    BlankReason,
    /// An `--at` is not `path:line`.
    #[error("`--at {0}` is not `path:line`, as `asymptote check` prints a finding")]
    At(String),
}

/// What the user does next about a command line the schemas refuse.
impl crate::outcome::remedy::Remedy for UpSchemaError {
    fn next(&self) -> crate::outcome::remedy::Next {
        match self {
            Self::BlankReason => crate::outcome::remedy::Next::Fix(
                "say why, `--reason \"<why>\"`: it lands on every record".to_owned(),
            ),
            Self::At(_) => crate::outcome::remedy::Next::Fix(
                "write `--at <path>:<line>`, as `asymptote check` prints a finding".to_owned(),
            ),
        }
    }
}

/// The arguments as the command line gives them, offered to the conversion.
struct UpArgumentsSchema<'a> {
    /// Why these findings are accepted: it lands on every record written.
    reason: String,
    /// Record only findings of this rule; repeat it for more.
    rule: Vec<String>,
    /// Record only findings in files this glob matches, relative to the product; repeat it for more.
    path: Vec<String>,
    /// Record only findings, and perf medians, of this tool; repeat it for more.
    tool: Vec<String>,
    /// Record only the new finding at PATH:LINE, as `check` prints it; repeat it for more.
    at: Vec<String>,
    /// Where the command runs, and how: whether it writes among it.
    edge: &'a crate::edge::Edge,
}

/// `path:line`, or `path:line:column` as `check` prints it, the column dropped.
fn at(text: &str) -> Result<At, UpSchemaError> {
    let refused = || UpSchemaError::At(text.to_owned());
    let number = |part: &str| part.parse::<u32>().ok().filter(|number| *number > 0); // ignore: idiomatic.arch/no-ok-on-result - a part that is no positive number is the answer: `path:line` has no column, and a last part that is none is refused by the caller
    let (rest, last) = text.rsplit_once(':').ok_or_else(refused)?;
    let last = number(last).ok_or_else(refused)?;
    let (path, line) = rest
        .rsplit_once(':')
        .and_then(|(path, line)| number(line).map(|line| (path, line)))
        .unwrap_or((rest, last));
    if path.is_empty() {
        return Err(refused());
    }
    Ok(At {
        path: normalize(&RepoPath(path.into())),
        line: LineNumber(line),
    })
}

impl TryFrom<UpArgumentsSchema<'_>> for UpRequest {
    type Error = UpSchemaError;

    fn try_from(schema: UpArgumentsSchema<'_>) -> Result<Self, Self::Error> {
        let edge = schema.edge;
        if schema.reason.trim().is_empty() {
            return Err(UpSchemaError::BlankReason);
        }
        Ok(Self {
            start: edge.start.clone(),
            reason: Reason(schema.reason.into()),
            selectors: Selectors {
                tools: schema
                    .tool
                    .into_iter()
                    .map(|tool| ToolId(tool.into()))
                    .collect(),
                rules: schema
                    .rule
                    .into_iter()
                    .map(|rule| RuleId(rule.into()))
                    .collect(),
                paths: schema
                    .path
                    .into_iter()
                    .map(|path| PathPattern(path.into()))
                    .collect(),
                at: schema
                    .at
                    .iter()
                    .map(|text| at(text))
                    .collect::<Result<_, _>>()?,
            },
            writing: edge.writing,
        })
    }
}

/// The service's request for the arguments.
///
/// # Errors
///
/// `BlankReason` for a reason that says nothing, `At` for an `--at` that is not `path:line`.
pub(super) fn request(
    reason: String,
    rule: Vec<String>,
    path: Vec<String>,
    tool: Vec<String>,
    at: Vec<String>,
    edge: &crate::edge::Edge,
) -> Result<UpRequest, UpSchemaError> {
    UpArgumentsSchema {
        reason,
        rule,
        path,
        tool,
        at,
        edge,
    }
    .try_into()
}

/// One perf record written.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(super) struct PerfRecordSchema {
    /// The tool.
    pub tool: String,
    /// The benchmark.
    pub benchmark: String,
    /// The measure.
    pub measure: String,
    /// The best, the median measured.
    pub best: f64,
}

/// What `up` prints.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(super) struct UpReportSchema {
    /// The records written.
    pub recorded: Vec<ViolationSchema>,
    /// The perf records written, in the same operation.
    pub perf: Vec<PerfRecordSchema>,
    /// The new violations left, which still fail `check`.
    pub left: Vec<ViolationSchema>,
    /// Whether the lock was written.
    pub written: bool,
    /// Whether it was compacted.
    pub compacted: bool,
}

impl std::fmt::Display for UpReportSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = if self.written {
            "recorded"
        } else {
            "would record"
        };
        for record in &self.recorded {
            writeln!(
                f,
                "{verb} {}: {}/{} ({})",
                record.path,
                record.tool,
                record.rule,
                record.count.unwrap_or_default()
            )?;
        }
        for record in &self.perf {
            writeln!(
                f,
                "{verb} perf {}/{}: {} best {}",
                record.tool, record.benchmark, record.measure, record.best
            )?;
        }
        for left in &self.left {
            writeln!(f, "left {}: {}/{}", place(left), left.tool, left.rule)?;
        }
        write!(
            f,
            "{} {verb}, {} new left",
            self.recorded.len(),
            self.left.len()
        )?;
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
pub(super) fn answer(report: &UpReport) -> UpReportSchema {
    report.into()
}

impl From<&UpReport> for UpReportSchema {
    fn from(report: &UpReport) -> Self {
        recorded(report)
    }
}

/// The answer printed for the service's report: what was recorded, the perf records written and the new findings left.
fn recorded(report: &UpReport) -> UpReportSchema {
    UpReportSchema {
        recorded: report
            .recorded
            .iter()
            .map(|written| record(written, ACCEPTED))
            .collect(),
        perf: report
            .perf
            .iter()
            .map(|record| PerfRecordSchema {
                tool: record.key.tool.0.to_string(),
                benchmark: record.key.benchmark.0.to_string(),
                measure: record.key.measure.0.to_string(),
                best: record.best.0,
            })
            .collect(),
        left: report
            .left
            .iter()
            .map(|found| violation(found, NEW))
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
    products: Vec<(String, Result<UpReport, UpError>)>,
) -> Result<(AnswerSchema<UpReportSchema>, Outcome), Outcome> {
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

impl crate::output::Encoded for UpReportSchema {
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
