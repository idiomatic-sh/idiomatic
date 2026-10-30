//! The shape of the data this output does not control: the one row every command prints a violation or a record as,
//! `check`'s findings, `up`'s records and what it leaves, and `down`'s records drained and recorded. Each command's
//! schemas fill it, and add beside it (flattened) only what that command alone says. At a workspace root, `check`,
//! `up` and `down` print each product's answer the one way: in a `members` list, each line prefixed with its path.

use asymptote_core::domain::operations::types::Record;
use asymptote_core::domain::violations::types::{Fingerprinted, Level, Place, Violation};

/// One violation, or one record of the lock, as every command prints it.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub(crate) struct ViolationSchema {
    /// The file, relative to the product's root.
    pub path: String,
    /// Its line, when it has one.
    pub line: Option<u32>,
    /// Its column, when it has one.
    pub column: Option<u32>,
    /// The tool.
    pub tool: String,
    /// The rule.
    pub rule: String,
    /// What the tool said.
    pub message: String,
    /// `new`, `resolved` or `accepted`.
    pub status: String,
    /// `note`, `warning` or `error`.
    pub level: String,
    /// What identifies it within its file, in hexadecimal; absent when its line could not be read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fingerprint: Option<String>,
    /// For a record: how many violations it accepts; for a resolved one, how many of them no longer occur.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub count: Option<u32>,
    /// For a resolved record: how many still do.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub remaining: Option<u32>,
}

/// The word a level is printed as.
pub(crate) fn level(level: Level) -> String {
    asymptote_core::domain::violations::service::word(level)
        .0
        .to_owned()
}

/// A fingerprint as it is printed: sixteen hexadecimal digits.
pub(crate) fn fingerprint(print: asymptote_core::domain::violations::types::Fingerprint) -> String {
    format!("{:016x}", print.0)
}

/// One violation of a run, with its `status`.
pub(crate) fn violation(violation: &Violation, status: &str) -> ViolationSchema {
    let (line, column) = match violation.place {
        Place::File => (None, None),
        Place::Line(line) => (Some(line.0), None),
        Place::At(line, column) => (Some(line.0), Some(column.0)),
    };
    ViolationSchema {
        path: violation.path.0.to_string(),
        line,
        column,
        tool: violation.tool.0.to_string(),
        rule: violation.rule.0.to_string(),
        message: violation.message.0.to_string(),
        status: status.to_owned(),
        level: level(violation.level),
        fingerprint: match violation.fingerprint {
            Fingerprinted::Known(print) => Some(fingerprint(print)),
            Fingerprinted::Unknown => None,
        },
        count: None,
        remaining: None,
    }
}

/// One record of the lock, with its `status` and the violations it accepts: a record has a file and no place in it.
pub(crate) fn record(record: &Record, status: &str) -> ViolationSchema {
    let key = &record.key;
    ViolationSchema {
        path: key.path.0.to_string(),
        line: None,
        column: None,
        tool: key.tool.0.to_string(),
        rule: key.rule.0.to_string(),
        message: record.message.0.to_string(),
        status: status.to_owned(),
        level: level(key.level),
        fingerprint: Some(fingerprint(key.fingerprint)),
        count: Some(record.count.0),
        remaining: None,
    }
}

/// Where a row is: `path:line:column`, `path:line` or the path, as a compiler prints it.
pub(crate) fn place(row: &ViolationSchema) -> String {
    match (row.line, row.column) {
        (Some(line), Some(column)) => format!("{}:{line}:{column}", row.path),
        (Some(line), None) => format!("{}:{line}", row.path),
        (None, _) => row.path.clone(),
    }
}

/// A line of one product's answer at a workspace root, prefixed with the product's path.
pub(crate) fn prefixed(path: &str, line: &str) -> String {
    format!("{path}: {line}")
}

/// One product's answer at a workspace root: its path, and beside it (flattened) what the command printed of it.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(crate) struct MemberSchema<R> {
    /// The product's path from the workspace root: `.` for the root itself.
    pub path: String,
    /// What the command printed of it.
    #[serde(flatten)]
    pub report: R,
}

/// What a command prints at a workspace root: each product's answer, in the order they ran.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub(crate) struct WorkspaceSchema<R> {
    /// Each product's answer.
    pub members: Vec<MemberSchema<R>>,
}

impl<R: std::fmt::Display> std::fmt::Display for WorkspaceSchema<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for member in &self.members {
            for line in member.report.to_string().lines() {
                writeln!(f, "{}", prefixed(&member.path, line))?;
            }
        }
        Ok(())
    }
}

/// What a ratchet command prints: one product's answer as it always was, or a workspace's.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
#[serde(untagged)]
pub(crate) enum AnswerSchema<R> {
    /// One product.
    Product(Box<R>),
    /// A workspace.
    Workspace(WorkspaceSchema<R>),
}

impl<R: std::fmt::Display> std::fmt::Display for AnswerSchema<R> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Product(report) => report.fmt(f),
            Self::Workspace(workspace) => workspace.fmt(f),
        }
    }
}

impl<R: std::fmt::Display + serde::Serialize> crate::output::Encoded for AnswerSchema<R> {
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

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
