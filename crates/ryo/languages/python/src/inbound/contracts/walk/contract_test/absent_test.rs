//! The `walk` contract's case of the relation that is no fact of the provider: which event answers a read is the rule pack's
//! selection over the events, the reads and the scopes, so the language answers `binding-resolution` as not read.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::walking::{walk_words, walked};
use ryo::languages::{AbsenceCause, FactAnswer, FilledColumns, NodeTable, Relation, Wanted};

/// The rows of the case of the relation the language does not fill.
const CASES: &[Case<Relation, Option<FactAnswer>, AbsenceCause>] = &[Case {
    name: CaseName("the_selection_among_events_is_not_a_fact_of_the_provider"),
    word: Word::Refuses,
    input: || Relation::BindingResolution,
    expect: Expect::Error(AbsenceCause::NotYetRead),
}];

/// What the language answers for a relation it does not fill, for one walked file: the cause it gives, or what it answered
/// instead; nothing when the file was not lent.
fn answered(relation: Relation) -> Result<Option<FactAnswer>, AbsenceCause> {
    let walk = crate::walk();
    let (mut table, mut scratch) = (NodeTable::default(), Default::default());
    let mut answer = None;
    let lent = walked(
        &walk,
        "input.py",
        b"x = 1\nprint(x)\n",
        &[relation],
        &mut table,
        &mut scratch,
        &mut |file| {
            let mut words = walk_words(&walk);
            answer = Some(file.facts.fill(
                relation,
                Wanted::Whole,
                &mut words,
                &mut FilledColumns::default(),
            ));
        },
    );
    match (lent, answer) {
        (Ok(()), Some(FactAnswer::Refused(cause))) => Err(cause),
        (Ok(()), other) => Ok(other),
        (Err(_), _) => Ok(None),
    }
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, answered);
}
