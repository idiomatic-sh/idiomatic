//! The accounted measure commands hold the run to the accounted inventory; the diagnostic ones are interim.
use super::value::MeasureCommand;
use crate::cases::{Case, CaseName, Expect, Word, run};

type Input = &'static str;
/// The scope, rendered as text.
type Value = &'static str;

const CASES: &[Case<Input, Value, ()>] = &[
    Case {
        name: CaseName("measure holds the run to the accounted inventory"),
        word: Word::Succeeds,
        input: || "measure",
        expect: Expect::Value("Accounted"),
    },
    Case {
        name: CaseName("measure-valid holds the run to the accounted inventory"),
        word: Word::Succeeds,
        input: || "measure-valid",
        expect: Expect::Value("Accounted"),
    },
    Case {
        name: CaseName("diagnostic-measure is interim"),
        word: Word::Succeeds,
        input: || "diagnostic-measure",
        expect: Expect::Value("Interim"),
    },
    Case {
        name: CaseName("diagnostic-measure-valid is interim"),
        word: Word::Succeeds,
        input: || "diagnostic-measure-valid",
        expect: Expect::Value("Interim"),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |command| {
        Ok(&*Box::leak(
            format!("{:?}", MeasureCommand(command).scope()).into_boxed_str(),
        ))
    });
}
