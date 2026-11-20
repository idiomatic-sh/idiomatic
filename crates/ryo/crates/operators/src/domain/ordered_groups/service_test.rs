//! The `ordered_groups` service's cases: the tie, side and emptiness rules of `as-of`, `group-min`, `group-max` and `group-ordinal`.
use super::service::{extreme, nearest, places};
use super::types::{GroupOrders, GroupRows, Place};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_relations::domain::relations::types::Integer;
use ryo_values::domain::plan_orders::types::{GroupEnd, OrderBound};

/// What a row asks of one key group's ascending orders.
#[derive(Debug)]
enum Asked {
    /// `as-of`: the rows nearest this probe on this side.
    Nearest(&'static [i64], i64, OrderBound),
    /// `group-min` or `group-max`: the rows at this end.
    Extreme(&'static [i64], GroupEnd),
    /// `group-ordinal`: the place of every row.
    Places(&'static [i64]),
}
type Input = Asked;
/// The places of the rows answered, in the group's order: for `group-ordinal`, the place each row is given.
type Value = std::borrow::Cow<'static, [u32]>;
type Failure = std::convert::Infallible;

/// The scope's first scenario: events at 2 and 5.
const EVENTS: &[i64] = &[2, 5];
/// Two events tied at 2, then one at 5.
const TIED: &[i64] = &[2, 2, 5];

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_probe_after_an_event_pairs_with_the_latest_event_before_it"),
        word: Word::Succeeds,
        input: || Asked::Nearest(EVENTS, 3, OrderBound::Before),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0])),
    },
    Case {
        name: CaseName("a_probe_after_every_event_pairs_with_the_last"),
        word: Word::Succeeds,
        input: || Asked::Nearest(EVENTS, 9, OrderBound::Before),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1])),
    },
    Case {
        name: CaseName("a_probe_before_every_event_has_no_row"),
        word: Word::Succeeds,
        input: || Asked::Nearest(EVENTS, 1, OrderBound::Before),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("an_event_at_the_probes_order_is_not_before_it"),
        word: Word::Succeeds,
        input: || Asked::Nearest(TIED, 5, OrderBound::Before),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0, 1])),
    },
    Case {
        name: CaseName("two_events_tied_for_latest_are_both_returned"),
        word: Word::Succeeds,
        input: || Asked::Nearest(TIED, 4, OrderBound::Before),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0, 1])),
    },
    Case {
        name: CaseName("at_or_before_includes_an_event_at_the_probes_order"),
        word: Word::Succeeds,
        input: || Asked::Nearest(TIED, 5, OrderBound::AtOrBefore),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[2])),
    },
    Case {
        name: CaseName("after_picks_the_earliest_event_after_it"),
        word: Word::Succeeds,
        input: || Asked::Nearest(&[2, 5, 7], 2, OrderBound::After),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1])),
    },
    Case {
        name: CaseName("at_or_after_includes_an_event_at_the_probes_order"),
        word: Word::Succeeds,
        input: || Asked::Nearest(&[2, 5, 7], 5, OrderBound::AtOrAfter),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1])),
    },
    Case {
        name: CaseName("a_probe_after_every_event_has_no_row_after_it"),
        word: Word::Succeeds,
        input: || Asked::Nearest(EVENTS, 5, OrderBound::After),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("a_group_with_no_event_has_no_row"),
        word: Word::Succeeds,
        input: || Asked::Nearest(&[], 5, OrderBound::AtOrAfter),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("group_max_ties_are_all_kept"),
        word: Word::Succeeds,
        input: || Asked::Extreme(&[1, 4, 4], GroupEnd::Greatest),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[1, 2])),
    },
    Case {
        name: CaseName("group_min_keeps_the_least_order"),
        word: Word::Succeeds,
        input: || Asked::Extreme(&[1, 4, 4], GroupEnd::Least),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0])),
    },
    Case {
        name: CaseName("group_min_ties_are_all_kept"),
        word: Word::Succeeds,
        input: || Asked::Extreme(&[1, 1, 4], GroupEnd::Least),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0, 1])),
    },
    Case {
        name: CaseName("an_empty_group_has_no_extreme"),
        word: Word::Succeeds,
        input: || Asked::Extreme(&[], GroupEnd::Least),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("the_first_rows_place_is_0"),
        word: Word::Succeeds,
        input: || Asked::Places(&[7, 9]),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0, 1])),
    },
    Case {
        name: CaseName("equal_orders_share_a_place"),
        word: Word::Succeeds,
        input: || Asked::Places(&[3, 3, 8, 8, 9]),
        expect: Expect::Value(std::borrow::Cow::Borrowed(&[0, 0, 2, 2, 4])),
    },
];

/// The places of a run's rows.
fn rows_of(run: GroupRows) -> Value {
    run.0
        .map(|row| u32::try_from(row).unwrap_or(u32::MAX))
        .collect::<Vec<u32>>()
        .into()
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let orders =
        |written: &[i64]| -> Vec<Integer> { written.iter().copied().map(Integer).collect() };
    Ok(match asked {
        Asked::Nearest(written, probe, bound) => rows_of(nearest(
            GroupOrders(&orders(written)),
            Integer(probe),
            bound,
        )),
        Asked::Extreme(written, end) => rows_of(extreme(GroupOrders(&orders(written)), end)),
        Asked::Places(written) => {
            let mut out: Vec<Place> = Vec::new();
            places(GroupOrders(&orders(written)), &mut out);
            out.into_iter()
                .map(|place| place.0)
                .collect::<Vec<u32>>()
                .into()
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
