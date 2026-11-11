//! A rule's inapplicable files are its own evidence beside the project identity: two identities of one request that differ only in
//! them are one identity to every check, among them the lookup of the frontier cache (`same_request`), and each keeps its own set.
use super::super::super::request_identity::service::RequestIdentity;
use super::super::{
    types::Agreement, types::Partition, types::PartitionSupply, types::RequiredPartitions,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::files::types::{
    CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds,
};
use std::sync::Arc;

fn corpus() -> Vec<CorpusInput> {
    ["a.rs", "b.rs"]
        .map(|path| {
            CorpusInput::Available(CorpusEntry {
                executable: ExecutablePermission::Unknown,
                path: FilePath(path.into()),
                size: FileSize(0),
                modified: Nanoseconds(0),
            })
        })
        .to_vec()
}

fn identities_that_differ_only_in_the_inapplicable_files_are_one_identity_and_keep_their_sets() {
    let files = corpus();
    let request = Arc::new(RequestIdentity::new(&files));
    let without = RequiredPartitions::from_request(&request, &files, &[], &[], &[], &[]).unwrap();
    let with =
        RequiredPartitions::from_request(&request, &files, &[], &[], &[FileId(1)], &[]).unwrap();
    assert!(
        without.inapplicable() != with.inapplicable(),
        "each identity keeps its own evidence"
    );
    assert!(without == with, "equality never reads the inapplicable set");
    assert_eq!(
        without.same_request(&with),
        Agreement::Same,
        "the frontier cache finds one acquisition for both"
    );
    assert_eq!(
        Partition::Project(without.clone()).supplies(&Partition::Project(with.clone())),
        PartitionSupply::Supplies,
        "a shared execution supplies either"
    );
    assert!(
        with.identity() == without,
        "the shared identity holds no rule's evidence"
    );
    assert!(with.identity().inapplicable().is_empty());
}

fn identities_of_two_requests_are_never_one_identity() {
    let files = corpus();
    let first = Arc::new(RequestIdentity::new(&files));
    let later = Arc::new(RequestIdentity::new(&files));
    let one =
        RequiredPartitions::from_request(&first, &files, &[], &[], &[FileId(1)], &[]).unwrap();
    let other =
        RequiredPartitions::from_request(&later, &files, &[], &[], &[FileId(1)], &[]).unwrap();
    assert_eq!(
        one.same_request(&other),
        Agreement::Different,
        "the same inapplicable files do not make two runs one identity"
    );
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    IdentitiesThatDifferOnlyInTheInapplicableFilesAreOneIdentityAndKeepTheirSets,
    IdentitiesOfTwoRequestsAreNeverOneIdentity,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "identities_that_differ_only_in_the_inapplicable_files_are_one_identity_and_keep_their_sets",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::IdentitiesThatDifferOnlyInTheInapplicableFilesAreOneIdentityAndKeepTheirSets
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("identities_of_two_requests_are_never_one_identity"),
        word: Word::Succeeds,
        input: || Scenario::IdentitiesOfTwoRequestsAreNeverOneIdentity,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::IdentitiesThatDifferOnlyInTheInapplicableFilesAreOneIdentityAndKeepTheirSets => {
            identities_that_differ_only_in_the_inapplicable_files_are_one_identity_and_keep_their_sets();
            Ok(())
        }
        Scenario::IdentitiesOfTwoRequestsAreNeverOneIdentity => {
            identities_of_two_requests_are_never_one_identity();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
