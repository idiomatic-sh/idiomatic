//! The shape of the data this command does not control: its arguments become the service's request, and
//! the service's report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::list_operators::types::ListOperatorsReport;

/// What the command prints: a line for each operator, its name and its summary apart by a tab, or the same as JSON, each operator
/// an object of the two.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The operators, in the catalog's order.
    pub operators: Vec<OperatorSchema>,
}

/// One operator of the answer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct OperatorSchema {
    /// The operator's name.
    pub name: String,
    /// What the operator does, in one line.
    pub summary: String,
}

impl From<ListOperatorsReport> for AnswerSchema {
    fn from(report: ListOperatorsReport) -> Self {
        Self {
            operators: report
                .operators
                .into_iter()
                .map(|operator| OperatorSchema {
                    name: operator.name.0.into_string(),
                    summary: operator.summary.0.into_string(),
                })
                .collect(),
        }
    }
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (place, operator) in self.operators.iter().enumerate() {
            let before = if place == 0 { "" } else { "\n" };
            write!(f, "{before}{}\t{}", operator.name, operator.summary)?;
        }
        writeln!(f)
    }
}

/// The answer printed for the service's report.
pub(super) fn answer(report: ListOperatorsReport) -> AnswerSchema {
    AnswerSchema::from(report)
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
