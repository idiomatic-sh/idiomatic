//! The shape of an accepted-differences file: the corpus digest it was written for and, per rule, a reason
//! and the unmatched findings it accepts, each by side, file, line and column.
use ryo_benchmark_contracts::domain::artifacts::types::digest::value::ArtifactDigest;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
use ryo_benchmark_contracts::domain::reports::types::AcceptanceReason;
use ryo_benchmark_contracts::domain::reports::types::{
    AcceptedDifferences, AcceptedRule, DifferenceSide, FindingPosition, LocatedDifference,
    SourceColumn, SourceLine,
};
use serde::Deserialize;

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;

/// The whole document; its description is for the reader.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct AcceptedSchema {
    /// Why the file exists, for the reader.
    #[expect(
        dead_code,
        reason = "the description is read by people, never by the comparison"
    )]
    description: String,
    /// The corpus digest the differences were observed on.
    corpus: String,
    /// The accepted differences of each rule.
    differences: Vec<RuleSchema>,
}

/// One rule's accepted differences.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct RuleSchema {
    /// The rule name.
    rule: String,
    /// Why the differences are accepted.
    reason: String,
    /// The accepted unmatched findings.
    findings: Vec<FindingSchema>,
}

/// One accepted unmatched finding.
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FindingSchema {
    /// Which contestant alone reports it.
    kind: SideSchema,
    /// The corpus-relative file.
    file: String,
    /// One-based line.
    line: usize,
    /// One-based Unicode-character column.
    column: usize,
}

/// The side of an unmatched finding as written.
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum SideSchema {
    /// Only the candidate reports it.
    Extra,
    /// Only the reference reports it.
    Missing,
}

/// Decode an accepted-differences file.
pub(in super::super) fn accepted(bytes: &[u8]) -> Result<AcceptedDifferences, Error> {
    let schema: AcceptedSchema =
        serde_json::from_slice(bytes).map_err(|error| Error::Invalid(error.to_string()))?;
    let rules = schema
        .differences
        .into_iter()
        .map(|entry| {
            Ok(AcceptedRule {
                rule: ryo::linter::RuleId::try_from(entry.rule.as_str())
                    .map_err(|error| Error::Invalid(error.to_string()))?,
                reason: AcceptanceReason(entry.reason),
                findings: entry
                    .findings
                    .into_iter()
                    .map(|finding| LocatedDifference {
                        side: match finding.kind {
                            SideSchema::Extra => DifferenceSide::Extra,
                            SideSchema::Missing => DifferenceSide::Missing,
                        },
                        file: ryo::linter::RelativePath(finding.file.into_boxed_str()),
                        start: FindingPosition {
                            line: SourceLine(finding.line),
                            column: SourceColumn(finding.column),
                        },
                    })
                    .collect(),
            })
        })
        .collect::<Result<_, Error>>()?;
    Ok(AcceptedDifferences {
        corpus: ArtifactDigest(schema.corpus),
        rules,
    })
}
