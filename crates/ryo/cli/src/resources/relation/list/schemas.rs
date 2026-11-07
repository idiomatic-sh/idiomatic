//! The shape of the data this command does not control: its arguments become the service's request, and
//! the service's report becomes the answer printed. Each refusal of a conversion is a `<Name>SchemaError`.

use ryo_core::inbound::use_cases::list_relations::types::ListRelationsReport;

/// What the command prints: a line for each relation, its name and its summary apart by a tab, or the same as JSON, each relation
/// an object of the two.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct AnswerSchema {
    /// The relations, in the catalog's order.
    pub relations: Vec<RelationSchema>,
}

/// One relation of the answer.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub(super) struct RelationSchema {
    /// The relation's name.
    pub name: String,
    /// What the relation holds, in one line.
    pub summary: String,
}

impl From<ListRelationsReport> for AnswerSchema {
    fn from(report: ListRelationsReport) -> Self {
        Self {
            relations: report
                .relations
                .into_iter()
                .map(|relation| RelationSchema {
                    name: relation.name.0.into_string(),
                    summary: relation.summary.0.to_owned(),
                })
                .collect(),
        }
    }
}

impl std::fmt::Display for AnswerSchema {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        for (place, relation) in self.relations.iter().enumerate() {
            let before = if place == 0 { "" } else { "\n" };
            write!(f, "{before}{}\t{}", relation.name, relation.summary)?;
        }
        writeln!(f)
    }
}

/// The answer printed for the service's report.
pub(super) fn answer(report: ListRelationsReport) -> AnswerSchema {
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
