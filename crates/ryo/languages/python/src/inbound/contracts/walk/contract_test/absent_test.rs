//! The `walk` contract's case of a relation the walk does not read: the language answers it as not read, with the cause.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::walking::{asked_set, walk_words, walked};
use ryo::languages::{AbsenceCause, FactAnswer, NodeTable, Relation, fact_tables};

/// The rows of the case of the relation the language does not fill.
const CASES: &[Case<Relation, Option<FactAnswer>, AbsenceCause>] = &[Case {
    name: CaseName("a_relation_the_walk_does_not_read_is_answered_as_not_read"),
    word: Word::Refuses,
    input: || Relation::FilesystemFile,
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
        &mut |mut file| {
            let mut words = walk_words(&walk).expect("each fixed name of the walk is stated once");
            let set = asked_set(&mut file, &[relation], &mut words);
            answer = Some(fact_tables::answer_of(&set, relation));
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
