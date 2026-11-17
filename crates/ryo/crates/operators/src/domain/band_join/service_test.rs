//! A band join is lowered in the plan's columns, its right columns read through its view, and answers the fan its answer and
//! keys say.
use super::service::{fan, lower};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::growth::types::Fan;
use ryo_values::domain::plan_parts::types::{
    BandAnswer, BandEdge, BandTest, ColumnIndex, JoinColumn, KeyFan, Side,
};

/// What a case lowers: the band, the answer, the view and the keys' fan.
type Input = (Vec<BandEdge>, BandAnswer, Vec<ColumnIndex>, KeyFan);
/// The right band column and the fan it answers.
type Value = (ColumnIndex, Fan);
/// It is no band join.
#[derive(Debug, PartialEq)]
struct Unlowered;
type Failure = Unlowered;

/// One edge from left column 0 to right column `right`.
fn edge(right: u32) -> Vec<BandEdge> {
    vec![BandEdge {
        left: ColumnIndex(0),
        test: BandTest::AtOrAbove,
        right: ColumnIndex(right),
    }]
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_nearest_band_answers_one_row_per_left_row"),
        word: Word::Succeeds,
        input: || (edge(1), BandAnswer::Nearest, Vec::new(), KeyFan::FileOnly),
        expect: Expect::Value((ColumnIndex(1), Fan::PerLeft)),
    },
    Case {
        name: CaseName("every_row_of_a_file_only_band_is_its_file_product"),
        word: Word::Succeeds,
        input: || (edge(1), BandAnswer::Every, Vec::new(), KeyFan::FileOnly),
        expect: Expect::Value((ColumnIndex(1), Fan::FileProduct)),
    },
    Case {
        name: CaseName("an_all_pairs_band_declares_its_pairs"),
        word: Word::Succeeds,
        input: || (edge(1), BandAnswer::Every, Vec::new(), KeyFan::AllPairs),
        expect: Expect::Value((ColumnIndex(1), Fan::DeclaredPairs)),
    },
    Case {
        name: CaseName("a_right_band_column_is_read_through_the_view"),
        word: Word::Succeeds,
        input: || {
            (
                edge(1),
                BandAnswer::Every,
                vec![ColumnIndex(4), ColumnIndex(7)],
                KeyFan::Shared,
            )
        },
        expect: Expect::Value((ColumnIndex(7), Fan::Repeats)),
    },
    Case {
        name: CaseName("a_band_column_past_the_view_is_no_band_join"),
        word: Word::Refuses,
        input: || {
            (
                edge(2),
                BandAnswer::Every,
                vec![ColumnIndex(4), ColumnIndex(7)],
                KeyFan::Shared,
            )
        },
        expect: Expect::Error(Unlowered),
    },
    Case {
        name: CaseName("an_empty_band_is_no_band_join"),
        word: Word::Refuses,
        input: || (Vec::new(), BandAnswer::Every, Vec::new(), KeyFan::Shared),
        expect: Expect::Error(Unlowered),
    },
];

fn subject((band, answer, view, keys): Input) -> Result<Value, Failure> {
    let output = [JoinColumn {
        side: Side::Left,
        column: ColumnIndex(0),
    }];
    let parts = lower(
        [&[ColumnIndex(0)], &[ColumnIndex(0)]],
        &band,
        answer,
        &output,
        &view,
        keys,
    )
    .ok_or(Unlowered)?;
    let right = parts.band.first().map(|edge| edge.right).ok_or(Unlowered)?;
    Ok((right, fan(&parts)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
