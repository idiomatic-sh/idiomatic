//! Typed storage for one real import membership row.
use super::super::interface::RelationsItem as _;
use super::item::{StandardLibraryImport, StandardLibraryImportRow};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, NodeId};
use crate::domain::terms::types::Symbol;

type Input = ();
type Value = [Bits; 3];
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("an_import_membership_keeps_its_occurrence_target_and_file"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([Bits(3), Bits(5), Bits(7)]),
}];

fn subject(_input: Input) -> Result<Value, Failure> {
    Ok(StandardLibraryImport::words(StandardLibraryImportRow {
        occurrence: NodeId(3),
        target: Symbol(5),
        file: FileId(7),
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
