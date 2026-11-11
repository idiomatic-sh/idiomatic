//! A project bucket is exact only when it holds every declared project file and no other.
use crate::application::file_dag::frontiers::service::exact_required_files;
use crate::application::file_dag::frontiers::types::FileAccounting;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::FileId;

/// The declared project files, and the files the bucket holds.
type Input = (&'static [u32], &'static [u32]);
type Value = FileAccounting;
type Failure = String;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_complete_bucket_requires_every_declared_project_file"),
        word: Word::Succeeds,
        input: || (&[0, 1], &[0, 1]),
        expect: Expect::Value(FileAccounting::Exact),
    },
    Case {
        name: CaseName("a_bucket_missing_a_declared_file_is_inexact"),
        word: Word::Succeeds,
        input: || (&[0, 1], &[0]),
        expect: Expect::Value(FileAccounting::Inexact),
    },
    Case {
        name: CaseName("a_bucket_with_an_undeclared_file_is_inexact"),
        word: Word::Succeeds,
        input: || (&[0, 1], &[0, 1, 2]),
        expect: Expect::Value(FileAccounting::Inexact),
    },
];

fn files(ids: &[u32]) -> Vec<FileId> {
    ids.iter().map(|id| FileId(*id)).collect()
}

/// Accounts the bucket against the declared files.
fn subject((required, held): Input) -> Result<Value, Failure> {
    Ok(exact_required_files(
        &files(required),
        &files(held).into_iter().collect(),
    ))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
