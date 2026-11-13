//! The `labels` service's cases: the number and the name the element table holds for a role, and the whole vocabulary.

use super::service::{field_bits, field_name, fields};
use super::types::Field;
use crate::cases::{Case, CaseName, Expect, Word, run};

/// What a row asks of the vocabulary.
#[derive(Clone, Copy, Debug)]
enum Given {
    /// One role: its number and its name.
    Role(Field),
    /// Every role: how many there are, and the name of the last.
    Vocabulary,
}
/// What a row is given.
type Input = Given;
/// A number and a name: a role's own, or the vocabulary's size and its last role's name.
type Value = (u16, &'static str);
/// Naming a role is never refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_edge_with_no_field_is_role_zero"),
        word: Word::Succeeds,
        input: || Given::Role(Field::None),
        expect: Expect::Value((0, "none")),
    },
    Case {
        name: CaseName("body_has_its_number_and_kebab_name"),
        word: Word::Succeeds,
        input: || Given::Role(Field::Body),
        expect: Expect::Value((1, "body")),
    },
    Case {
        name: CaseName("the_vocabulary_lists_every_role_in_number_order"),
        word: Word::Succeeds,
        input: || Given::Vocabulary,
        expect: Expect::Value((70, "asname")),
    },
];

/// Reads the role or the vocabulary the row names.
fn subject(given: Input) -> Result<Value, Failure> {
    Ok(match given {
        Given::Role(field) => (field_bits(field).0, field_name(field).0),
        Given::Vocabulary => {
            let every: Vec<Field> = fields().collect();
            let count = u16::try_from(every.len()).unwrap_or(u16::MAX);
            let numbered = every
                .iter()
                .enumerate()
                .all(|(at, field)| usize::from(field_bits(*field).0) == at);
            let last = every.last().map_or("", |field| field_name(*field).0);
            (if numbered { count } else { 0 }, last)
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
