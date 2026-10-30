//! The shape of the data this command does not control: its arguments become the service's request, and the service's
//! report becomes the answer printed.

use tastemaker_core::domain::proofs::types::Proof;
use tastemaker_core::inbound::use_cases::validate::types::{
    Proving, ValidateReport, ValidateRequest, VersionFilter,
};

use crate::outcome::Outcome;

/// The arguments as the command line gives them, offered to the conversion.
struct ValidateArgumentsSchema<'a> {
    /// Prove only the versions whose id holds one of these texts, `local/rule/no-todo-comments@0.1.0`.
    only: Vec<String>,
    /// Where the command runs, and how.
    edge: &'a crate::edge::Edge,
}

impl From<ValidateArgumentsSchema<'_>> for ValidateRequest {
    fn from(schema: ValidateArgumentsSchema<'_>) -> Self {
        let edge = schema.edge;
        Self {
            start: edge.start.clone(),
            only: schema
                .only
                .into_iter()
                .map(|only| VersionFilter(only.into_boxed_str()))
                .collect(),
        }
    }
}

/// The service's request for the arguments.
pub(super) fn request(only: Vec<String>, edge: &crate::edge::Edge) -> ValidateRequest {
    ValidateArgumentsSchema { only, edge }.into()
}

/// One version that does not hold.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct FailedSchema {
    /// The version.
    pub id: String,
    /// What is wrong.
    pub problems: Vec<String>,
}

/// What `validate` prints.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// How many versions hold.
    pub proven: usize,
    /// How many proofs were lookups.
    pub kept: usize,
    /// The versions that do not hold.
    pub failed: Vec<FailedSchema>,
    /// A failure when a version does not hold.
    #[serde(skip)]
    pub outcome: Outcome,
}

impl crate::outcome::Ending for AnswerSchema {
    fn ending(&self) -> Outcome {
        self.outcome
    }
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for failed in &self.failed {
            writeln!(f, "{}", failed.id)?;
            for problem in &failed.problems {
                writeln!(f, "  {problem}")?;
            }
        }
        writeln!(
            f,
            "{} version(s) hold, {} do not; {} proof(s) kept from an earlier run",
            self.proven,
            self.failed.len(),
            self.kept
        )
    }
}

/// The answer printed for the service's report, and the outcome: a failure when a version does not hold.
pub(super) fn answer(report: ValidateReport) -> AnswerSchema {
    let mut answer = AnswerSchema {
        proven: 0,
        kept: 0,
        failed: Vec::new(),
        outcome: Outcome::Success,
    };
    for proved in report.proved {
        if proved.proving == Proving::Kept {
            answer.kept += 1;
        }
        match proved.proof {
            Proof::Proven => answer.proven += 1,
            Proof::Failed(problems) => answer.failed.push(FailedSchema {
                id: proved.subject.id.0.to_string(),
                problems: problems
                    .into_iter()
                    .map(|problem| problem.0.to_string())
                    .collect(),
            }),
        }
    }
    if !answer.failed.is_empty() {
        answer.outcome = Outcome::Failure;
    }
    answer
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
