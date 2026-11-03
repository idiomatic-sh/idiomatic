//! The `completion` service's cases, on values alone.

use super::service::{completion, name_kind};
use crate::domain::elements::service::{clear_elements, kind_names};
use crate::domain::elements::types::Elements;
use ryo_relations::domain::relations::constants::NO_SYMBOL;
use ryo_relations::domain::relations::types::{FileId, KindBits};
use crate::domain::symbols::service::{clear_symbols, derived_of};
use crate::domain::symbols::types::ByteSlice;
use crate::domain::symbols::types::Symbols;
use ryo_relations::domain::terms::types::NameText;

use crate::cases::{Case, CaseName, Expect, Word, run};

/// A scenario: what a row asks of the service.
type Input = fn() -> usize;
/// What a scenario answers: a count.
type Value = usize;
type Failure = ();

fn a_named_kind_is_found_by_its_number() -> usize {
    let (mut elements, mut symbols) = (Elements::default(), Symbols::default());
    let source = ByteSlice(b"fn");
    clear_elements(&mut elements);
    clear_symbols(&mut symbols);
    {
        let mut completing = completion(&mut elements, &mut symbols, source, FileId(0));
        name_kind(&mut completing, KindBits(2), NameText("name"));
        name_kind(&mut completing, KindBits(2), NameText("name"));
        name_kind(&mut completing, KindBits(0), NameText("root"));
    }
    let names = kind_names(&elements);
    let found = names.get(2).copied();
    usize::from(
        found.is_some_and(|symbol| symbol != NO_SYMBOL)
            && names.get(1).copied() == Some(NO_SYMBOL)
            && derived_of(&symbols).0 == b"nameroot",
    )
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_named_kind_is_found_by_its_number"),
    word: Word::Succeeds,
    input: || a_named_kind_is_found_by_its_number,
    expect: Expect::Value(1),
}];

fn subject(scenario: Input) -> Result<Value, Failure> {
    Ok(scenario())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
