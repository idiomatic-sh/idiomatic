//! The `statistics` service's cases: a file of a sample that measured no unit reads as statistics of its stage, which is not
//! the same as no statistics; bytes that hold no statistics are absent, never an error, whatever they are. What a sample
//! and a second sample learn, and how their units read back, are the cases of the `learning` part.
mod learning;

use super::service::decoded;
use super::types::{LearnStage, StatisticsBytes};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::packs::constants::ENGINE_VERSION;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};

/// The file a row decodes, as it is written: `{version}` stands for the version of the engine under test.
#[derive(Clone, Copy, Debug)]
struct Written(&'static str);

/// What bytes held.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Held {
    /// Statistics of a first sample, with this many units.
    First(usize),
    /// Statistics of a second sample, with this many units.
    Second(usize),
    /// They are absent.
    Absent,
}

type Input = Written;
type Value = Held;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("no_bytes_are_no_statistics"),
        word: Word::Succeeds,
        input: || Written(""),
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("bytes_that_do_not_read_are_absent_never_an_error"),
        word: Word::Succeeds,
        input: || Written("not statistics"),
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("a_first_sample_that_measured_no_unit_is_statistics_of_stage_first"),
        word: Word::Succeeds,
        input: || Written("ryo-statistics\n{version}\nfirst\n"),
        expect: Expect::Value(Held::First(0)),
    },
    Case {
        name: CaseName("a_second_sample_that_measured_no_unit_is_statistics_of_stage_second"),
        word: Word::Succeeds,
        input: || Written("ryo-statistics\n{version}\nsecond\n"),
        expect: Expect::Value(Held::Second(0)),
    },
    Case {
        name: CaseName("statistics_of_another_engine_version_are_absent"),
        word: Word::Succeeds,
        input: || Written("ryo-statistics\n{version}.1\nfirst\n"),
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("a_stage_no_sample_has_is_absent"),
        word: Word::Succeeds,
        input: || Written("ryo-statistics\n{version}\nthird\n"),
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("a_header_whose_last_line_is_not_ended_is_absent"),
        word: Word::Succeeds,
        input: || Written("ryo-statistics\n{version}\nfirst"),
        expect: Expect::Value(Held::Absent),
    },
    Case {
        name: CaseName("a_header_followed_by_bytes_that_are_no_units_is_absent"),
        word: Word::Succeeds,
        input: || Written("ryo-statistics\n{version}\nfirst\nunits"),
        expect: Expect::Value(Held::Absent),
    },
];

fn subject(written: Input) -> Result<Value, Failure> {
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let bytes = written.0.replace("{version}", ENGINE_VERSION);
    let held = decoded(StatisticsBytes(bytes.as_bytes()), &kinds);
    Ok(
        held.map_or(Held::Absent, |statistics| match statistics.stage {
            LearnStage::First => Held::First(statistics.units.len()),
            LearnStage::Second => Held::Second(statistics.units.len()),
        }),
    )
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
