//! The `corpus_copies` service's cases, on values alone.

use ryo_values::domain::files::types::FileBytes;

use super::service::copied;
use super::types::{
    CopyCount, CopyDigits, CopyNaming, CorpusInputs, Extension, ItemName, ListShape, Repeat,
};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Lists the one file of an item.
    Input,
    /// Lists three padded copies of an item.
    Copies,
    /// Lists an input written three times over.
    Repeated,
}

/// What a row of the cases does.
type Input = Scenario;
/// One if the list is the files the row names, and none other.
type Value = usize;
type Failure = ();

fn inputs() -> CorpusInputs {
    CorpusInputs(
        [(
            ItemName("group-count".to_owned()),
            FileBytes(b"ab".to_vec()),
        )]
        .into_iter()
        .collect(),
    )
}

fn subject(scenario: Scenario) -> Value {
    let extension = Extension("rs".to_owned());
    let files = |naming, copies, repeat| {
        copied(
            &inputs(),
            &extension,
            ListShape {
                naming,
                copies: CopyCount(copies),
                repeat: Repeat(repeat),
            },
        )
    };
    match scenario {
        Scenario::Input => {
            let listed = files(CopyNaming::Input, 1, 1);
            usize::from(
                listed.len() == 1
                    && &*listed[0].path.0 == "group_count/input.rs"
                    && listed[0].bytes.0 == b"ab",
            )
        }
        Scenario::Copies => {
            let listed = files(
                CopyNaming::Copy {
                    digits: CopyDigits(3),
                },
                3,
                1,
            );
            usize::from(
                listed.iter().map(|file| &*file.path.0).collect::<Vec<_>>()
                    == [
                        "group_count/copy_000.rs",
                        "group_count/copy_001.rs",
                        "group_count/copy_002.rs",
                    ],
            )
        }
        Scenario::Repeated => {
            let listed = files(CopyNaming::Input, 1, 3);
            usize::from(listed[0].bytes.0 == b"ababab")
        }
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_item_has_one_file_named_input_in_the_folder_of_its_name"),
        word: Word::Succeeds,
        input: || Scenario::Input,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("copies_are_numbered_from_zero_padded_to_the_digits_asked"),
        word: Word::Succeeds,
        input: || Scenario::Copies,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("an_input_repeated_has_its_bytes_one_after_the_other"),
        word: Word::Succeeds,
        input: || Scenario::Repeated,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
