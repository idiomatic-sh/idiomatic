//! A visited set answers whether a walk met an entity already, forgets every entity at a reset, and resets at the cost of
//! what the walk met: small walks after one huge walk cost what they met, not the huge walk's room each.
use super::value::{ResetWork, VisitedSet};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::table_scratch::types::SetMembership;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};

/// How many entities the huge walk meets.
const HUGE: i64 = 100_000;
/// How many small walks follow it.
const SMALL_WALKS: i64 = 1_000;
/// How many entities each small walk meets.
const SMALL: i64 = 3;

/// The walks a case runs.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One walk meets one entity twice.
    Again,
    /// A huge walk meets an entity, the set resets, and the next walk meets it.
    AfterReset,
    /// A huge walk, then many small walks, each reset after.
    SmallAfterHuge,
}
type Input = Given;
type Value = Observation;
type Failure = ();

/// What the case observed.
#[derive(Debug, PartialEq, Eq)]
enum Observation {
    /// Whether the last meeting found the entity met already.
    Met(SetMembership),
    /// The slots the small walks' resets emptied.
    Work(ResetWork),
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_entity_met_again_in_one_walk_is_in_the_set"),
        word: Word::Succeeds,
        input: || Given::Again,
        expect: Expect::Value(Observation::Met(SetMembership::In)),
    },
    Case {
        name: CaseName("an_entity_met_before_a_reset_is_out_after_it"),
        word: Word::Succeeds,
        input: || Given::AfterReset,
        expect: Expect::Value(Observation::Met(SetMembership::Out)),
    },
    Case {
        name: CaseName("small_walks_after_a_huge_walk_reset_at_the_cost_of_what_they_met"),
        word: Word::Succeeds,
        input: || Given::SmallAfterHuge,
        expect: Expect::Value(Observation::Work(ResetWork(3000))),
    },
];

/// The entity numbered `at`.
fn entity(at: i64) -> Cell {
    Cell::Number(Integer(at))
}

/// Meets every entity numbered in `from..to` once, then resets.
fn walked(set: &mut VisitedSet, from: i64, to: i64) {
    for at in from..to {
        let _first = set.meet(entity(at));
    }
    set.reset();
}

fn subject(given: Input) -> Result<Value, Failure> {
    let mut set = VisitedSet::default();
    Ok(match given {
        Given::Again => {
            let _first = set.meet(entity(7));
            Observation::Met(set.meet(entity(7)))
        }
        Given::AfterReset => {
            walked(&mut set, 0, HUGE);
            Observation::Met(set.meet(entity(0)))
        }
        Given::SmallAfterHuge => {
            walked(&mut set, 0, HUGE);
            let before = set.reset_work();
            for walk in 0..SMALL_WALKS {
                walked(&mut set, walk * SMALL, (walk + 1) * SMALL);
            }
            Observation::Work(ResetWork(set.reset_work().0 - before.0))
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
