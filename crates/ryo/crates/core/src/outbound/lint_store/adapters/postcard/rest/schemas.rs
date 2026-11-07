//! The rest document of the postcard store: its wire shapes and their conversion from and into the core's tables.

use std::borrow::Cow;
use ryo_values::domain::descriptions::types::RelationName;
use ryo_values::domain::records::types::GoverningRules;
use ryo_values::domain::stored_facts::types::ArenaBytes;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stored_facts::types::FileNotes;
use ryo_values::domain::stored_facts::types::LanguageNumber;
use ryo_values::domain::stored_facts::types::NoteBytes;
use ryo_values::domain::stored_facts::types::PartialAt;
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::domain::stored_facts::types::PartialKind;
use ryo_values::domain::stored_facts::types::PartialScope;
use ryo_values::domain::stored_facts::types::StoredPartial;
use ryo_values::domain::stored_findings::types::StoredFileNumber;
use ryo_values::domain::stores::types::StoredCovered;
use ryo_values::domain::stores::types::StoredHanded;
use ryo_values::domain::stores::types::StoredNotes;
use ryo_values::domain::stores::types::StoredText;
use super::schemas::{StringsSchema, TextSchema, as_bytes, named, text};
use super::super::proofs::schemas::{FrontierProofSchema, proof_row, proof_schema};

/// Bytes of a file only the core reads, its names' text: one row for each file that has some.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct BytesSchema<'a> {
    /// The file, by its place among the files.
    pub(in crate::outbound::lint_store::adapters::postcard) file: u32,
    /// The bytes.
    #[serde(borrow, serialize_with = "as_bytes")]
    pub(in crate::outbound::lint_store::adapters::postcard) bytes: Cow<'a, [u8]>,
}

/// What a file's language noted of it, which only that language reads: one row for each file that has some.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct NotesSchema<'a> {
    /// The file, by its place among the files.
    pub(in crate::outbound::lint_store::adapters::postcard) file: u32,
    /// The language that wrote the notes, by its place among the ones linked.
    pub(in crate::outbound::lint_store::adapters::postcard) language: u32,
    /// The notes.
    #[serde(borrow, serialize_with = "as_bytes")]
    pub(in crate::outbound::lint_store::adapters::postcard) bytes: Cow<'a, [u8]>,
}

/// The rules that govern a file read in part: one row for each such file.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct CoveredSchema<'a> {
    /// The file, by its place among the files.
    pub(in crate::outbound::lint_store::adapters::postcard) file: u32,
    /// The rules, by their place.
    pub(in crate::outbound::lint_store::adapters::postcard) governing: Cow<'a, [u32]>,
}

/// A relation of a file read in part: one row for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct PartialSchema {
    /// The file, by its place among the files.
    pub(in crate::outbound::lint_store::adapters::postcard) file: u32,
    /// The relation.
    pub(in crate::outbound::lint_store::adapters::postcard) relation: TextSchema,
    /// Why it is read in part.
    pub(in crate::outbound::lint_store::adapters::postcard) cause: TextSchema,
    /// The kind of the rows that could be missing; empty when any could be.
    pub(in crate::outbound::lint_store::adapters::postcard) lacking: TextSchema,
    /// The file the rows missing are about.
    pub(in crate::outbound::lint_store::adapters::postcard) at: u32,
}

/// Typed named-root readiness beside the existing frontier bucket rows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) enum ObservationReadinessSchema {
    Unknown(TextSchema),
    NoQuery,
}

#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct ObservationSchema {
    pub(in crate::outbound::lint_store::adapters::postcard) file: u32,
    pub(in crate::outbound::lint_store::adapters::postcard) edition: u32,
    pub(in crate::outbound::lint_store::adapters::postcard) layout: [u8; 16],
    pub(in crate::outbound::lint_store::adapters::postcard) slots: u32,
    pub(in crate::outbound::lint_store::adapters::postcard) complete: Vec<u64>,
    pub(in crate::outbound::lint_store::adapters::postcard) skipped: Vec<u64>,
    pub(in crate::outbound::lint_store::adapters::postcard) refusals: Vec<(u32, ObservationReadinessSchema)>,
}

/// What the files handed the steps that read across files and the project but their words.
#[derive(Clone, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
// ignore: idiomatic.arch/no-deep-stored-types - the nesting is the byte layout of the store file already written: flattening it would change the postcard bytes of every kept frontier proof
pub(in crate::outbound::lint_store::adapters::postcard) struct RestSchema<'a> {
    pub(in crate::outbound::lint_store::adapters::postcard) observations: Vec<ObservationSchema>,
    /// Source identity and coverage for each payload row, in the rest section
    /// so existing bucket byte framing and unchanged-bucket copying stay exact.
    pub(in crate::outbound::lint_store::adapters::postcard) proofs: Vec<FrontierProofSchema>,
    /// Every text the rows name, each once.
    #[serde(borrow)]
    pub(in crate::outbound::lint_store::adapters::postcard) strings: Vec<Cow<'a, str>>,
    /// The text each file's names made.
    #[serde(borrow)]
    pub(in crate::outbound::lint_store::adapters::postcard) texts: Vec<BytesSchema<'a>>,
    /// What each file's language noted of it.
    #[serde(borrow)]
    pub(in crate::outbound::lint_store::adapters::postcard) notes: Vec<NotesSchema<'a>>,
    /// The rules that govern each file read in part.
    pub(in crate::outbound::lint_store::adapters::postcard) covered: Vec<CoveredSchema<'a>>,
    /// The relations of each file read in part.
    pub(in crate::outbound::lint_store::adapters::postcard) partials: Vec<PartialSchema>,
}

pub(in crate::outbound::lint_store::adapters::postcard) fn rest_of<'a>(stored: &'a StoredHanded) -> RestSchema<'a> {
    let mut strings = StringsSchema::default();
    let bytes = |file: StoredFileNumber, bytes: &'a [u8]| BytesSchema {
        file: file.0,
        bytes: Cow::Borrowed(bytes),
    };
    let mut partials = Vec::new();
    for each in &stored.covered {
        for partial in &each.partials {
            partials.push(PartialSchema {
                file: each.file.0,
                relation: text(&mut strings, &partial.relation_name.0),
                cause: text(&mut strings, &partial.cause.0),
                lacking: text(&mut strings, &partial.lacking_kind.0),
                at: match partial.at {
                    PartialScope::OneFile(file) => file.0,
                    PartialScope::Repository => u32::MAX,
                },
            });
        }
    }
    let observations = stored
        .observations
        .iter()
        .map(|each| ObservationSchema {
            file: each.file.0,
            edition: each.root.version.0,
            layout: each.root.layout.0.0,
            slots: each.root.slots.0,
            complete: each.root.complete.0.clone(),
            skipped: each.root.skipped.0.clone(),
            refusals: each
                .root
                .refusals
                .iter()
                .map(|refusal| {
                    (
                        refusal.slot.0,
                        match &refusal.reason {
                            ryo_values::domain::records::types::FrontierRefusalReason::Unknown(
                                reason,
                            ) => ObservationReadinessSchema::Unknown(text(&mut strings, &reason.0)),
                            ryo_values::domain::records::types::FrontierRefusalReason::NoQuery => {
                                ObservationReadinessSchema::NoQuery
                            }
                        },
                    )
                })
                .collect(),
        })
        .collect();
    RestSchema {
        observations,
        proofs: stored.proofs.iter().map(proof_schema).collect(),
        texts: stored
            .texts
            .iter()
            .map(|each| bytes(each.file, &each.text.0))
            .collect(),
        notes: stored
            .notes
            .iter()
            .map(|each| NotesSchema {
                file: each.file.0,
                language: u32::try_from(each.notes.language.0).unwrap_or(u32::MAX),
                bytes: Cow::Borrowed(each.notes.notes.0.as_slice()),
            })
            .collect(),
        covered: stored
            .covered
            .iter()
            .map(|each| CoveredSchema {
                file: each.file.0,
                governing: Cow::Borrowed(&each.governing.0),
            })
            .collect(),
        partials,
        strings: strings.texts,
    }
}

/// What the files handed but their words, from its section.
pub(in crate::outbound::lint_store::adapters::postcard) fn handed_of(rest: &RestSchema<'_>) -> Result<StoredHanded, postcard::Error> {
    let strings = &rest.strings;
    let mut proofs = rest
        .proofs
        .iter()
        .map(proof_row)
        .collect::<Result<Vec<_>, _>>()?;
    proofs.sort_by_key(|row| (row.file.0, row.bucket.0));
    if proofs
        .windows(2)
        .any(|pair| pair[0].file == pair[1].file && pair[0].bucket == pair[1].bucket)
    {
        return Err(postcard::Error::DeserializeBadEncoding);
    }
    let mut covered = rest
        .covered
        .iter()
        .map(|each| StoredCovered {
            file: StoredFileNumber(each.file),
            governing: GoverningRules(each.governing.to_vec()),
            partials: Vec::new(),
        })
        .collect::<Vec<_>>();
    for each in &rest.partials {
        let at = covered
            .binary_search_by(|row| row.file.0.cmp(&each.file))
            .map_err(|_not_covered| postcard::Error::DeserializeBadEncoding)?;
        if let Some(row) = covered.get_mut(at) {
            row.partials.push(StoredPartial {
                relation_name: RelationName(named(strings, each.relation)?),
                cause: PartialCause(named(strings, each.cause)?),
                lacking_kind: PartialKind(named(strings, each.lacking)?),
                at: if each.at == u32::MAX {
                    PartialScope::Repository
                } else {
                    PartialScope::OneFile(PartialAt(each.at))
                },
            });
        }
    }
    Ok(StoredHanded(
        ryo_values::domain::stores::types::HandedRows {
            proofs,
            observations: rest
                .observations
                .iter()
                .map(|each| {
                    use ryo_values::domain::records::types::ObservationReadinessCount;
                    use ryo_values::domain::records::types::ObservationReadinessLayout;
                    use ryo_values::domain::records::types::ObservationReadinessSlot;
                    use ryo_values::domain::records::types::ObservationReadinessVersion;
                    use ryo_values::domain::records::types::ObservationReason;
                    use ryo_values::domain::records::types::{
                        FrontierReadiness, FrontierRefusal, FrontierRefusalReason,
                    };
                    Ok(ryo_values::domain::stores::types::StoredFrontierReadiness {
                        file: StoredFileNumber(each.file),
                        root: std::sync::Arc::new(FrontierReadiness {
                            version: ObservationReadinessVersion(each.edition),
                            layout: ObservationReadinessLayout(Digest(each.layout)),
                            slots: ObservationReadinessCount(each.slots),
                            complete: ryo_values::domain::records::types::ReadinessBits(
                                each.complete.clone(),
                            ),
                            skipped: ryo_values::domain::records::types::ReadinessBits(
                                each.skipped.clone(),
                            ),
                            refusals: each
                                .refusals
                                .iter()
                                .map(|(slot, reason)| {
                                    Ok(FrontierRefusal {
                                        slot: ObservationReadinessSlot(*slot),
                                        reason: match *reason {
                                            ObservationReadinessSchema::Unknown(reason) => {
                                                FrontierRefusalReason::Unknown(ObservationReason(
                                                    named(strings, reason)?,
                                                ))
                                            }
                                            ObservationReadinessSchema::NoQuery => {
                                                FrontierRefusalReason::NoQuery
                                            }
                                        },
                                    })
                                })
                                .collect::<Result<_, postcard::Error>>()?,
                        }),
                    })
                })
                .collect::<Result<_, postcard::Error>>()?,
            frontier: Vec::new(),
            texts: rest
                .texts
                .iter()
                .map(|each| StoredText {
                    file: StoredFileNumber(each.file),
                    text: ArenaBytes(each.bytes.to_vec()),
                })
                .collect(),
            notes: rest
                .notes
                .iter()
                .map(|each| StoredNotes {
                    file: StoredFileNumber(each.file),
                    notes: FileNotes {
                        language: LanguageNumber(
                            usize::try_from(each.language).unwrap_or(usize::MAX),
                        ),
                        notes: NoteBytes(each.bytes.to_vec()),
                    },
                })
                .collect(),
            covered,
        },
    ))
}

/// What the files of `stored` handed but their words, as their section's bytes.
pub(in crate::outbound::lint_store::adapters::postcard) fn rest_bytes(stored: &StoredHanded) -> Result<Vec<u8>, postcard::Error> {
    // The room the bytes take, at least, so they are written once and never moved as the section grows.
    let room = stored
        .texts
        .iter()
        .map(|each| each.text.0.len() + 10)
        .sum::<usize>()
        + stored
            .notes
            .iter()
            .map(|each| each.notes.notes.0.len() + 20)
            .sum::<usize>();
    postcard::to_extend(&rest_of(stored), Vec::with_capacity(room))
}

/// What the files handed but their words, from the section's bytes.
pub(in crate::outbound::lint_store::adapters::postcard) fn rest_from(bytes: &[u8]) -> Option<StoredHanded> {
    // A section that does not read back holds nothing the run can use again.
    let Ok(rest) = postcard::from_bytes::<RestSchema<'_>>(bytes) else {
        return None;
    };
    match handed_of(&rest) {
        Ok(handed) => Some(handed),
        Err(_) => None,
    }
}
