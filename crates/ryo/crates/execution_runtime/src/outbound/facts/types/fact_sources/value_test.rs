//! The sources of one file: built over one tree source and further ones, refused when two serve one relation.

use super::value::FactSources;
use crate::outbound::facts::errors::FactsError;
use crate::outbound::facts::fixtures::sources::SourceFake;
use crate::outbound::facts::types::fact_sources::value::FactSource;
use ryo_language::domain::elements::types::Elements;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};

/// What the tree source and each further source serve.
type Input = (&'static [Relation], &'static [&'static [Relation]]);
/// How many sources the set holds, the tree's included.
type Value = usize;
type Failure = FactsError;

fn subject((tree, others): Input) -> Result<Value, Failure> {
    let mut tree = SourceFake::serving(tree);
    let mut further: Vec<SourceFake> = others
        .iter()
        .map(|each| SourceFake::serving(each))
        .collect();
    let mut others: Vec<&mut dyn FactSource> = further
        .iter_mut()
        .map(|each| -> &mut dyn FactSource { each })
        .collect();
    let mut sources = FactSources::read(&mut tree, &mut others)?;
    Ok(sources.each().count())
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_tree_source_alone_is_one_source"),
        word: Word::Succeeds,
        input: || (&[Relation::Function], &[]),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_further_source_of_another_relation_is_reached"),
        word: Word::Succeeds,
        input: || (&[Relation::Function], &[&[Relation::Comment]]),
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("two_sources_serving_one_relation_are_refused"),
        word: Word::Refuses,
        input: || {
            (
                &[Relation::Comment, Relation::Function],
                &[&[Relation::Function, Relation::Comment]],
            )
        },
        expect: Expect::Error(FactsError::Overlap {
            relation: Relation::Function,
        }),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    let elements = Elements::default();
    assert_eq!(
        FactSources::unread(&elements, &mut []).map(|mut sources| sources.each().count()),
        Ok(0)
    );
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
