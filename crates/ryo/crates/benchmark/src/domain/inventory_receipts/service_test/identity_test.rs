//! The registered input, cohort and declaration identities bind both native sides.
use ryo_benchmark_contracts::domain::documents::types::observation_document::value::{ContextMode, ContextRun, ListedContext};
use ryo_benchmark_contracts::domain::documents::types::text::value::{FileMetadataDigest, PathText, Sha256Text};

use super::super::{
    errors::InventoryReceiptsError,
    fixtures::{self, Documents},
    types::ReceiptCounts,
};
use super::{COHORT, DECLARED, counts};
use crate::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::registered_inputs::errors::RegisteredInputsError,
};
use ryo_benchmark_contracts::domain::documents::types::case_document::value::{CaseContextIdentity, EntryState, FileMetadata};

type Input = Documents;
type Value = ReceiptCounts;
type Failure = InventoryReceiptsError;

fn subject(input: Input) -> Result<Value, Failure> {
    super::subject(input)
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a case that never declared file metadata keeps its receipt contract"),
        word: Word::Succeeds,
        input: fixtures::unspecified_legacy,
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("the registered project scope binds both raw sides"),
        word: Word::Succeeds,
        input: || fixtures::project_scope(&["src"], EntryState::Regular, None),
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("an explicit cohort on the case and both rows is accepted"),
        word: Word::Succeeds,
        input: || fixtures::with_selected_rules(Some(COHORT), [Some(COHORT), Some(COHORT)]),
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("observation declarations carried by both rows are accepted"),
        word: Word::Succeeds,
        input: || {
            fixtures::with_rule_observations(Some(DECLARED), [Some(DECLARED), Some(DECLARED)])
        },
        expect: Expect::Value(counts(1, 0, 0)),
    },
    Case {
        name: CaseName("a case without its metadata declaration is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(
                || fixtures::with_file_metadata(false),
                |d| d.0[0].file_metadata = FileMetadata::Undeclared,
            )
        },
        expect: Expect::Error(InventoryReceiptsError::Inputs(
            RegisteredInputsError::InputObservationsMissing,
        )),
    },
    Case {
        name: CaseName("a case without its metadata identity is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(
                || fixtures::with_file_metadata(false),
                |d| d.0[0].file_metadata_sha256 = FileMetadataDigest::Undeclared,
            )
        },
        expect: Expect::Error(InventoryReceiptsError::FileMetadataDigestDiffers),
    },
    Case {
        name: CaseName("rows without the metadata identity are refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(
                || fixtures::with_file_metadata(false),
                |d| {
                    for row in &mut d.1 {
                        row.file_metadata_sha256 = FileMetadataDigest::Undeclared;
                    }
                },
            )
        },
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a changed project scope no longer matches its registered identity"),
        word: Word::Refuses,
        input: || fixtures::project_scope(&["vendor", "src"], EntryState::Regular, None),
        expect: Expect::Error(InventoryReceiptsError::ProjectInputsDigestDiffers),
    },
    Case {
        name: CaseName("a changed project entry no longer matches its registered identity"),
        word: Word::Refuses,
        input: || fixtures::project_scope(&["src"], EntryState::Missing, None),
        expect: Expect::Error(InventoryReceiptsError::ProjectInputsDigestDiffers),
    },
    Case {
        name: CaseName("a stale project identity on a row is refused"),
        word: Word::Refuses,
        input: || fixtures::project_scope(&["src"], EntryState::Regular, Some("stale-identity")),
        expect: Expect::Error(InventoryReceiptsError::ObservationStale),
    },
    Case {
        name: CaseName("a cohort narrowed on the case is refused"),
        word: Word::Refuses,
        input: || fixtures::with_selected_rules(Some(&["A001"]), [Some(COHORT), Some(COHORT)]),
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a cohort missing on a row is refused"),
        word: Word::Refuses,
        input: || fixtures::with_selected_rules(Some(COHORT), [None, Some(COHORT)]),
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a cohort missing on the case is refused"),
        word: Word::Refuses,
        input: || fixtures::with_selected_rules(None, [Some(COHORT), Some(COHORT)]),
        expect: Expect::Error(InventoryReceiptsError::SelectedCohortDiffers),
    },
    Case {
        name: CaseName("a changed observation declaration on a row is refused"),
        word: Word::Refuses,
        input: || fixtures::with_rule_observations(Some(DECLARED), [Some(DECLARED), Some("other")]),
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
    },
    Case {
        name: CaseName("an observation declaration missing on a row is refused"),
        word: Word::Refuses,
        input: || fixtures::with_rule_observations(Some(DECLARED), [None, Some(DECLARED)]),
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
    },
    Case {
        name: CaseName("an observation declaration missing on the case is refused"),
        word: Word::Refuses,
        input: || fixtures::with_rule_observations(None, [Some(DECLARED), Some(DECLARED)]),
        expect: Expect::Error(InventoryReceiptsError::ObservationDeclarationsDiffer),
    },
    Case {
        name: CaseName("a project context receipt in an isolated run is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.1[0].context_manifest = ContextRun::Listed(ListedContext {
                    mode: ContextMode::ListedDisk,
                    logical_root: PathText("root".into()),
                    actual_root: PathText("/cases/root".into()),
                    manifest_hash: Sha256Text("hash".into()),
                })
            })
        },
        expect: Expect::Error(InventoryReceiptsError::UnexpectedProjectContext),
    },
    Case {
        name: CaseName("a project context receipt that no longer matches its identity is refused"),
        word: Word::Refuses,
        input: || {
            fixtures::edited(fixtures::passing, |d| {
                d.0[0].project_context_mode = ContextMode::ListedDisk;
                d.0[0].context_identity = CaseContextIdentity::Listed(fixtures::listed_context());
            })
        },
        expect: Expect::Error(InventoryReceiptsError::StaleProjectContext),
    },
];

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
