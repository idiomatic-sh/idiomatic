//! The `lexical_facts` service's cases: each filler's rows for a source, against the rows today's reader of the relation writes
//! for the same source.

use super::fixtures::{CONSTRUCTS, Compared, compared};
use super::service::{
    defines_rows, implicit_symbol_rows, qualified_name_rows, reference_rows, resolves_rows, scope_rows,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo::languages;

/// The relation and the source.
type Input = (Filler, &'static str);
/// How the filler's rows stand against today's.
type Value = Compared;
type Failure = ();

/// The fillers of this concept.
#[derive(Clone, Copy, Debug)]
enum Filler {
    /// `scope`.
    Scope,
    /// `defines`.
    Defines,
    /// `reference`.
    Reference,
    /// `resolves`.
    Resolves,
    /// `qualified-name`.
    QualifiedName,
    /// `implicit-symbol`.
    ImplicitSymbol,
}

/// The fillers' rows equal today's readers' rows: `rows` of them, none differing, no cause.
const fn equal(rows: usize) -> Expect<Value, Failure> {
    Expect::Value(Compared { rows, differing: Vec::new(), causes: Vec::new() })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("scopes_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::Scope, CONSTRUCTS),
        expect: equal(17),
    },
    Case {
        name: CaseName("definitions_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::Defines, CONSTRUCTS),
        expect: equal(80),
    },
    Case {
        name: CaseName("references_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::Reference, CONSTRUCTS),
        expect: equal(106),
    },
    Case {
        name: CaseName("resolutions_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::Resolves, CONSTRUCTS),
        expect: equal(78),
    },
    Case {
        name: CaseName("qualified_names_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::QualifiedName, CONSTRUCTS),
        expect: equal(117),
    },
    Case {
        name: CaseName("implicit_symbols_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::ImplicitSymbol, CONSTRUCTS),
        expect: equal(173),
    },
];

fn subject((filler, source): Input) -> Value {
    match filler {
        Filler::Scope => compared::<languages::Scope>(source, |view, words, rows| scope_rows(view, words, rows)),
        Filler::Defines => compared::<languages::Defines>(source, |view, words, rows| defines_rows(view, words, rows)),
        Filler::Reference => {
            compared::<languages::Reference>(source, |view, words, rows| reference_rows(view, words, rows))
        }
        Filler::Resolves => compared::<languages::Resolves>(source, |view, words, rows| resolves_rows(view, words, rows)),
        Filler::QualifiedName => {
            compared::<languages::QualifiedName>(source, |view, words, rows| qualified_name_rows(view, words, rows))
        }
        Filler::ImplicitSymbol => {
            compared::<languages::ImplicitSymbol>(source, |view, words, rows| implicit_symbol_rows(view, words, rows))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
