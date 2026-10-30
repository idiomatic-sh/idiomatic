//! What every `project` adapter must pass.

use super::super::errors::ManifestReadingError;
use super::super::interface::ManifestReadingPort;
use super::super::types::Claim;
use crate::cases::{Case, CaseName, Expect, Word, run, selects};
use ryo::languages::{FilePath, ManifestFile, ManifestText};

/// What a row of the contract asks of an adapter.
#[derive(Clone, Copy, Debug)]
pub(crate) enum Ask {
    /// Whether it claims the file of its own that the sample names.
    ClaimsItsFile,
    /// Whether it claims a file of each name it asks the repository for.
    ClaimsEveryNameItAsksFor,
    /// Whether it claims a file no package manager writes.
    ClaimsAReadme,
    /// What it declares for the sample's manifest.
    ReadsItsFile,
    /// What it declares for the sample's unreadable manifest.
    ReadsABrokenFile,
}

/// What an adapter is checked with: a file it claims, a manifest it reads, and one it cannot.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Sample {
    /// The path of a manifest the adapter claims.
    pub path: &'static str,
    /// Its text, which the adapter reads.
    pub readable: &'static str,
    /// Text it cannot read.
    pub broken: &'static str,
    /// Further manifests of the repository the adapter is given with the first.
    pub others: &'static [(&'static str, &'static str)],
}

/// What a row answers: whether the adapter claimed the file, or what it declared.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Answer {
    /// Its claim.
    Claimed(Claim),
    /// That it read the manifest.
    Read,
}

/// What a row expects when the manifest cannot be read.
type Failure = ManifestReadingError;

/// The contract's rows: a name, one case word, a question and the expected answer or error variant. Every implementation runs them.
pub(crate) const CASES: &[Case<Ask, Answer, Failure>] = &[
    Case {
        name: CaseName("an_adapter_claims_the_file_of_its_package_manager"),
        word: Word::Succeeds,
        input: || Ask::ClaimsItsFile,
        expect: Expect::Value(Answer::Claimed(Claim::Reads)),
    },
    Case {
        name: CaseName("an_adapter_claims_a_file_of_every_name_it_asks_the_repository_for"),
        word: Word::Succeeds,
        input: || Ask::ClaimsEveryNameItAsksFor,
        expect: Expect::Value(Answer::Claimed(Claim::Reads)),
    },
    Case {
        name: CaseName("an_adapter_skips_a_file_no_package_manager_writes"),
        word: Word::Succeeds,
        input: || Ask::ClaimsAReadme,
        expect: Expect::Value(Answer::Claimed(Claim::Skips)),
    },
    Case {
        name: CaseName("a_manifest_it_claims_is_read"),
        word: Word::Succeeds,
        input: || Ask::ReadsItsFile,
        expect: Expect::Value(Answer::Read),
    },
    Case {
        name: CaseName("a_manifest_it_cannot_read_is_unreadable"),
        word: Word::Refuses,
        input: || Ask::ReadsABrokenFile,
        expect: Expect::Refusal(|error| {
            selects(matches!(error, ManifestReadingError::Unreadable { .. }))
        }),
    },
];

/// Runs the contract's rows against one implementation: the fake, or an adapter, built by `build` over the manifests of a repository,
/// with a sample of its own manifests.
///
/// # Panics
///
/// When the implementation breaks the contract.
pub(crate) fn check<P>(build: impl Fn(Vec<ManifestFile>) -> P, sample: Sample)
where
    P: ManifestReadingPort,
{
    let declared = |text: &'static str| -> Result<Answer, Failure> {
        let own = ManifestFile {
            path: FilePath(sample.path.into()),
            text: ManifestText(text.into()),
        };
        let mut all = vec![own.clone()];
        all.extend(sample.others.iter().map(|(path, written)| ManifestFile {
            path: FilePath((*path).into()),
            text: ManifestText((*written).into()),
        }));
        build(all).declared(&own).map(|_| Answer::Read)
    };
    let adapter = build(Vec::new());
    let subject = |ask: Ask| -> Result<Answer, Failure> {
        match ask {
            Ask::ClaimsItsFile => Ok(Answer::Claimed(
                adapter.claims(&FilePath(sample.path.into())),
            )),
            Ask::ClaimsEveryNameItAsksFor => Ok(Answer::Claimed(
                if adapter
                    .manifests()
                    .iter()
                    .all(|name| adapter.claims(&FilePath(name.0.into())) == Claim::Reads)
                {
                    Claim::Reads
                } else {
                    Claim::Skips
                },
            )),
            Ask::ClaimsAReadme => Ok(Answer::Claimed(
                adapter.claims(&FilePath("README.md".into())),
            )),
            Ask::ReadsItsFile => declared(sample.readable),
            Ask::ReadsABrokenFile => declared(sample.broken),
        }
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
