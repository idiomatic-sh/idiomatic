//! The proofs document of the postcard store: its wire shapes and their conversion from and into the core's tables.

use ryo_values::domain::stored_facts::types::{Bucket, Digest, PartialKind};
use ryo_values::domain::stored_findings::types::StoredFileNumber;

/// Which scope of the repository a part of a frontier proof is still uncertain in, and what is missing of it.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) enum FrontierAffectedSchema {
    CurrentFileAny,
    CurrentFileKinds { column: u32, kinds: Vec<String> },
    RepositoryAny,
    RepositoryKinds { column: u32, kinds: Vec<String> },
}

/// Where a frontier's words came from: the file, the bucket, the rule and its input, the rule's digest by its place in the
/// section's table of digests, and what was uncertain of it, none when complete.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct FrontierProofSchema {
    file: u32,
    bucket: u32,
    rule: u32,
    input: u32,
    digest: u32,
    partial: Option<Vec<FrontierAffectedSchema>>,
}

/// The section of what the files of `stored` handed but their words.
/// The proof's rule digest is written as `digest`, its place in the section's table of digests.
pub(in crate::outbound::lint_store::adapters::postcard) fn proof_schema(
    row: &ryo_values::domain::stores::types::StoredFrontierProof,
    digest: u32,
) -> FrontierProofSchema {
    use ryo_values::domain::records::types::FrontierCoverage;
    FrontierProofSchema {
        file: row.file.0,
        bucket: row.bucket.0,
        rule: row.proof.rule.0,
        input: row.proof.input.0,
        digest,
        partial: match &row.proof.coverage {
            FrontierCoverage::Complete => None,
            FrontierCoverage::Partial(affected) => {
                Some(affected.iter().map(affected_schema).collect())
            }
        },
    }
}

fn affected_schema(
    cause: &ryo_values::domain::records::types::FrontierAffected,
) -> FrontierAffectedSchema {
    use ryo_values::domain::records::types::{FrontierAffected, FrontierMissing};
    match cause {
        FrontierAffected::CurrentFile(FrontierMissing::Any) => {
            FrontierAffectedSchema::CurrentFileAny
        }
        FrontierAffected::CurrentFile(FrontierMissing::Kinds { column, kinds }) => {
            FrontierAffectedSchema::CurrentFileKinds {
                column: column.0,
                kinds: kinds.iter().map(|kind| kind.0.to_string()).collect(),
            }
        }
        FrontierAffected::Repository(FrontierMissing::Any) => FrontierAffectedSchema::RepositoryAny,
        FrontierAffected::Repository(FrontierMissing::Kinds { column, kinds }) => {
            FrontierAffectedSchema::RepositoryKinds {
                column: column.0,
                kinds: kinds.iter().map(|kind| kind.0.to_string()).collect(),
            }
        }
    }
}

fn affected_row(
    cause: &FrontierAffectedSchema,
) -> ryo_values::domain::records::types::FrontierAffected {
    use ryo_values::domain::plan_parts::types::ColumnIndex;
    use ryo_values::domain::records::types::{FrontierAffected, FrontierMissing};
    use ryo_values::domain::stored_facts::types::PartialKind;
    let kinds_of = |kinds: &[String]| {
        kinds
            .iter()
            .map(|kind| PartialKind(kind.as_str().into()))
            .collect()
    };
    match cause {
        FrontierAffectedSchema::CurrentFileAny => {
            FrontierAffected::CurrentFile(FrontierMissing::Any)
        }
        FrontierAffectedSchema::CurrentFileKinds { column, kinds } => {
            FrontierAffected::CurrentFile(FrontierMissing::Kinds {
                column: ColumnIndex(*column),
                kinds: kinds_of(kinds),
            })
        }
        FrontierAffectedSchema::RepositoryAny => FrontierAffected::Repository(FrontierMissing::Any),
        FrontierAffectedSchema::RepositoryKinds { column, kinds } => {
            FrontierAffected::Repository(FrontierMissing::Kinds {
                column: ColumnIndex(*column),
                kinds: kinds_of(kinds),
            })
        }
    }
}

/// The proof `row` names, its rule digest read from `digests`, the section's table of digests.
pub(in crate::outbound::lint_store::adapters::postcard) fn proof_row(
    row: &FrontierProofSchema,
    digests: &[[u8; 16]],
) -> Result<ryo_values::domain::stores::types::StoredFrontierProof, postcard::Error> {
    use ryo_values::domain::records::types::{FrontierCoverage, FrontierProof};
    let coverage = match &row.partial {
        None => FrontierCoverage::Complete,
        Some(affected) => {
            let values = affected.iter().map(affected_row).collect::<Vec<_>>();
            if ryo_values::domain::records::service::is_well_formed(&values)
                == ryo_values::domain::records::types::FrontierShape::Malformed
            {
                return Err(postcard::Error::DeserializeBadEncoding);
            }
            FrontierCoverage::Partial(values)
        }
    };
    Ok(ryo_values::domain::stores::types::StoredFrontierProof {
        file: StoredFileNumber(row.file),
        bucket: Bucket(row.bucket),
        proof: FrontierProof {
            rule: ryo_values::domain::records::types::ProducerRule(row.rule),
            input: ryo_values::domain::records::types::FrontierOrdinal(row.input),
            rule_digest: Digest(
                *digests
                    .get(
                        usize::try_from(row.digest)
                            .map_err(|_| postcard::Error::DeserializeBadEncoding)?,
                    )
                    .ok_or(postcard::Error::DeserializeBadEncoding)?,
            ),
            coverage,
        },
    })
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
