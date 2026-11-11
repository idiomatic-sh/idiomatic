//! A rule's inapplicable files are its own evidence beside the project identity: two identities of one request that differ only in
//! them are one identity to every check, among them the lookup of the frontier cache (`same_request`), and each keeps its own set.
use crate::application::file_dag::program::admission::{
    service::Partition, types::Agreement, types::PartitionSupply, types::RequiredPartitions,
};
use crate::application::file_dag::program::request_identity::service::RequestIdentity;
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

/// Two identities over the same corpus: whether they come from one request, and each one's inapplicable files.
#[derive(Clone, Copy, Debug)]
struct Pair {
    one_request: bool,
    left: &'static [u32],
    right: &'static [u32],
}

/// How the two identities compare.
#[derive(Debug, PartialEq)]
struct Compared {
    /// Whether the two inapplicable sets are equal.
    same_inapplicable: bool,
    /// Whether the identities are equal.
    equal: bool,
    /// What the frontier cache lookup answers.
    agreement: Agreement,
    /// Whether a project execution of the left supplies the right.
    supply: PartitionSupply,
    /// Whether the right one's shared identity equals the left one.
    shared_identity_is_left: bool,
    /// Whether the right one's shared identity holds no inapplicable file.
    shared_identity_bare: bool,
}

type Input = Pair;
type Value = Compared;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "identities_that_differ_only_in_the_inapplicable_files_are_one_identity_and_keep_their_sets",
        ),
        word: Word::Succeeds,
        input: || Pair {
            one_request: true,
            left: &[],
            right: &[1],
        },
        expect: Expect::Value(Compared {
            same_inapplicable: false,
            equal: true,
            agreement: Agreement::Same,
            supply: PartitionSupply::Supplies,
            shared_identity_is_left: true,
            shared_identity_bare: true,
        }),
    },
    Case {
        name: CaseName("identities_of_two_requests_are_never_one_identity"),
        word: Word::Succeeds,
        input: || Pair {
            one_request: false,
            left: &[1],
            right: &[1],
        },
        expect: Expect::Value(Compared {
            same_inapplicable: true,
            equal: false,
            agreement: Agreement::Different,
            supply: PartitionSupply::Lacks,
            shared_identity_is_left: false,
            shared_identity_bare: true,
        }),
    },
];

fn ids(files: &[u32]) -> Vec<FileId> {
    files.iter().map(|file| FileId(*file)).collect()
}

/// Builds both identities and compares them by every check that reads them.
fn subject(pair: Input) -> Result<Value, Failure> {
    let files = corpus();
    let first = Arc::new(RequestIdentity::new(&files));
    let later = if pair.one_request {
        Arc::clone(&first)
    } else {
        Arc::new(RequestIdentity::new(&files))
    };
    let left = RequiredPartitions::from_request(&first, &files, &[], &[], &ids(pair.left), &[])
        .map_err(|error| format!("{error:?}"))?;
    let right = RequiredPartitions::from_request(&later, &files, &[], &[], &ids(pair.right), &[])
        .map_err(|error| format!("{error:?}"))?;
    Ok(Compared {
        same_inapplicable: left.inapplicable() == right.inapplicable(),
        equal: left == right,
        agreement: left.same_request(&right),
        supply: Partition::Project(left.clone()).supplies(&Partition::Project(right.clone())),
        shared_identity_is_left: right.identity() == left,
        shared_identity_bare: right.identity().inapplicable().is_empty(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
