//! Initialization requests and printable reports.
use tastemaker_core::inbound::use_cases::init::types::{InitReport, InitRequest};
/// The initialized directory is the explicit root or current working directory.
pub(super) fn request(edge: &crate::edge::Edge) -> InitRequest {
    InitRequest {
        target: edge.start.clone(),
        writing: edge.writing,
    }
}
/// The initialization result.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// Created files, relative to the target directory.
    pub written: Vec<String>,
    /// Whether no writes occurred.
    pub dry_run: bool,
}
/// What a new workspace's author runs next, a line each.
pub(super) const NEXT: [&str; 3] = [
    "tm explain          what applies here, and every topic",
    "tm explain --help   how to write a rule, spec or template",
    "tm check            the findings",
];

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let verb = crate::output::dry_run::written(self.dry_run);
        for file in &self.written {
            writeln!(f, "{verb} {file}")?;
        }
        if !self.dry_run {
            for step in NEXT {
                writeln!(f, "next: {step}")?;
            }
        }
        Ok(())
    }
}
/// Turns the service report into command output.
pub(super) fn answer(report: InitReport, dry_run: bool) -> AnswerSchema {
    AnswerSchema {
        dry_run,
        ..report.into()
    }
}
impl From<InitReport> for AnswerSchema {
    fn from(report: InitReport) -> Self {
        Self {
            written: report
                .written
                .into_iter()
                .map(|path| path.0.to_string())
                .collect(),
            dry_run: false,
        }
    }
}
#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

impl crate::output::Encoded for AnswerSchema {
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
