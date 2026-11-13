//! Public schema and typed row words of an engine-owned input fact.
use super::super::interface::RelationsItem as _;
use super::{
    constants,
    item::{FilesystemExecutable, FilesystemExecutableColumn, FilesystemExecutableRow},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::relations::types::{Bits, FileId, Flag};

/// What the schema and the words of both flags answer.
#[derive(Debug, PartialEq, Eq)]
struct Observed {
    name: &'static str,
    path_column: &'static str,
    executable_column: &'static str,
    lowered: [Bits; 2],
    raised: [Bits; 2],
}

type Input = ();
type Value = Observed;
type Failure = ();

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("schema_and_rows_preserve_file_then_boolean_without_an_unknown_sentinel"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(Observed {
        name: "filesystem.executable",
        path_column: "path",
        executable_column: "executable",
        lowered: [Bits(7), Bits(0)],
        raised: [Bits(7), Bits(1)],
    }),
}];

fn words(executable: Flag) -> [Bits; 2] {
    FilesystemExecutable::words(FilesystemExecutableRow {
        path: FileId(7),
        executable,
    })
}

fn subject(_: Input) -> Result<Value, Failure> {
    Ok(Observed {
        name: constants::name(),
        path_column: constants::column_name(FilesystemExecutableColumn::Path),
        executable_column: constants::column_name(FilesystemExecutableColumn::Executable),
        lowered: words(Flag::Lowered),
        raised: words(Flag::Raised),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
