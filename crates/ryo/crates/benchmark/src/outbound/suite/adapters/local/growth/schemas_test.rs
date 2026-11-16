//! What the growth table accepts and refuses, one row of `CASES` for each input.
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = &'static str;
/// The rules, the sizes, the frames and the operations decoded.
type Value = (usize, usize, usize, usize);
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a rule over an operation in a frame at one size is a table"),
        word: Word::Succeeds,
        input: || {
            "file = \"case.py\"\nsizes = [1000]\n[[rules]]\nrule = \"F401\"\nprofile = \"profiles/F401\"\n[frames.module]\nhead = \"\"\nindent = \"\"\ntail = \"\"\n[[operations]]\nname = \"assign\"\nframes = [\"module\"]\nsegments = [{ lines = [\"v = 1\\n\"], fixed = 0, per-size = 1 }]\n"
        },
        expect: Expect::Value((1, 1, 1, 1)),
    },
    Case {
        name: CaseName("an operation in a frame the table does not define is refused"),
        word: Word::Refuses,
        input: || {
            "file = \"case.py\"\nsizes = [1000]\n[[rules]]\nrule = \"F401\"\nprofile = \"profiles/F401\"\n[frames.module]\nhead = \"\"\nindent = \"\"\ntail = \"\"\n[[operations]]\nname = \"assign\"\nframes = [\"class\"]\nsegments = [{ lines = [\"v = 1\\n\"], fixed = 0, per-size = 1 }]\n"
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("an odd size is refused"),
        word: Word::Refuses,
        input: || {
            "file = \"case.py\"\nsizes = [999]\n[[rules]]\nrule = \"F401\"\nprofile = \"profiles/F401\"\n[frames.module]\nhead = \"\"\nindent = \"\"\ntail = \"\"\n[[operations]]\nname = \"assign\"\nframes = [\"module\"]\nsegments = [{ lines = [\"v = 1\\n\"], fixed = 0, per-size = 1 }]\n"
        },
        expect: Expect::Error(()),
    },
];
/// Decodes one table and counts its rules, sizes, frames and operations.
fn subject(text: Input) -> Result<Value, Failure> {
    super::growth(text)
        .map(|table| {
            (
                table.rules.len(),
                table.sizes.len(),
                table.frames.len(),
                table.operations.len(),
            )
        })
        .map_err(|_refused| ())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
