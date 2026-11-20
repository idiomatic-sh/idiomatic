//! The `binding_facts` service's cases: each filler's rows for a source, against the rows today's reader of the relation writes
//! for the same source; and the files today's readers do not answer (a file with a syntax error, a name with more events than
//! today's chain limit), which the fillers answer in full. That the selection among events is no fact of the provider is a
//! case of the walk's contract, where the language answers the relation.

use super::service::{binding_event_rows, binding_read_rows, binding_shadow_rows, binding_use_rows};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lexical_facts::fixtures::{CONSTRUCTS, Compared, compared};
use crate::fixtures::model_rows::{Counted, counted};
use crate::domain::lexical_facts::service::{
    defines_rows, implicit_symbol_rows, qualified_name_rows, reference_rows, resolves_rows, scope_rows,
};
use ryo::languages;

/// The relation and the source.
type Input = (Filler, &'static str);
/// How the filler's rows stand against today's.
type Value = Compared;
type Failure = ();

/// The fillers of this concept.
#[derive(Clone, Copy, Debug)]
enum Filler {
    /// `binding-event`.
    Event,
    /// `binding-read`.
    Read,
    /// `binding-use`.
    Use,
    /// `binding-shadow`.
    Shadow,
}

/// The fillers' rows equal today's readers' rows: `rows` of them, none differing, no cause.
const fn equal(rows: usize) -> Expect<Value, Failure> {
    Expect::Value(Compared { rows, differing: Vec::new(), causes: Vec::new() })
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("events_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::Event, CONSTRUCTS),
        expect: equal(192),
    },
    Case {
        name: CaseName("reads_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::Read, CONSTRUCTS),
        expect: equal(109),
    },
    Case {
        name: CaseName("uses_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::Use, CONSTRUCTS),
        expect: equal(88),
    },
    Case {
        name: CaseName("shadows_of_every_construct_equal_todays"),
        word: Word::Succeeds,
        input: || (Filler::Shadow, CONSTRUCTS),
        expect: equal(11),
    },
];

fn subject((filler, source): Input) -> Result<Value, Failure> {
    Ok(match filler {
        Filler::Event => {
            compared::<languages::BindingEvent>(source, |view, words, rows| binding_event_rows(view, words, rows))
        }
        Filler::Read => compared::<languages::BindingRead>(source, |view, words, rows| binding_read_rows(view, words, rows)),
        Filler::Use => compared::<languages::BindingUse>(source, |view, words, rows| binding_use_rows(view, words, rows)),
        Filler::Shadow => {
            compared::<languages::BindingShadow>(source, |view, words, rows| binding_shadow_rows(view, words, rows))
        }
    })
}

/// The ten relations' rows of `source` against today's: how many today's readers wrote in all, and every differing row
/// after its relation's name.
fn every(source: &'static str) -> Compared {
    let parts = [
        ("scope", compared::<languages::Scope>(source, |view, words, rows| scope_rows(view, words, rows))),
        ("defines", compared::<languages::Defines>(source, |view, words, rows| defines_rows(view, words, rows))),
        ("reference", compared::<languages::Reference>(source, |view, words, rows| reference_rows(view, words, rows))),
        ("resolves", compared::<languages::Resolves>(source, |view, words, rows| resolves_rows(view, words, rows))),
        (
            "qualified-name",
            compared::<languages::QualifiedName>(source, |view, words, rows| qualified_name_rows(view, words, rows)),
        ),
        (
            "implicit-symbol",
            compared::<languages::ImplicitSymbol>(source, |view, words, rows| implicit_symbol_rows(view, words, rows)),
        ),
        ("binding-event", subject((Filler::Event, source)).unwrap_or_default()),
        ("binding-read", subject((Filler::Read, source)).unwrap_or_default()),
        ("binding-use", subject((Filler::Use, source)).unwrap_or_default()),
        ("binding-shadow", subject((Filler::Shadow, source)).unwrap_or_default()),
    ];
    let mut all = Compared::default();
    for (relation, part) in parts {
        all.rows += part.rows;
        all.differing.extend(part.differing.iter().map(|row| format!("{relation} {row}")));
        all.causes.extend(part.causes.iter().map(|cause| format!("{relation} {cause}")));
    }
    all
}

/// The rows of the cases over every relation at once.
const EVERY: &[Case<&'static str, Value, Failure>] = &[
    Case {
        name: CaseName("the_scope_fixture"),
        word: Word::Succeeds,
        input: || include_str!("../../../tests/fixtures/relations/scope/input.py"),
        expect: equal(290),
    },
    Case {
        name: CaseName("the_defines_fixture"),
        word: Word::Succeeds,
        input: || include_str!("../../../tests/fixtures/relations/defines/input.py"),
        expect: equal(325),
    },
    Case {
        name: CaseName("the_reference_fixture"),
        word: Word::Succeeds,
        input: || include_str!("../../../tests/fixtures/relations/reference/input.py"),
        expect: equal(248),
    },
    Case {
        name: CaseName("the_resolves_fixture"),
        word: Word::Succeeds,
        input: || include_str!("../../../tests/fixtures/relations/resolves/input.py"),
        expect: equal(223),
    },
    Case {
        name: CaseName("the_qualified_name_fixture"),
        word: Word::Succeeds,
        input: || include_str!("../../../tests/fixtures/relations/qualified_name/input.py"),
        expect: equal(182),
    },
    Case {
        name: CaseName("the_binding_resolution_fixture"),
        word: Word::Succeeds,
        input: || include_str!("../../../tests/fixtures/relations/binding_resolution/input.py"),
        expect: equal(197),
    },
    Case {
        name: CaseName("type_parameters_of_functions_classes_and_aliases"),
        word: Word::Succeeds,
        input: || TYPE_PARAMETERS,
        expect: equal(312),
    },
    Case {
        name: CaseName("annotations_postponed_by_the_future_import"),
        word: Word::Succeeds,
        input: || FUTURE_ANNOTATIONS,
        expect: equal(217),
    },
];

/// PEP 695: type parameters of a function, a class and a type alias, with bounds, defaults and uses.
const TYPE_PARAMETERS: &str = "\
def first[T](items: list[T]) -> T:
    return items[0]


class Box[T: int = str, *Ts, **P]:
    item: T

    def get[U](self, other: U) -> tuple[T, U]:
        return self.item, other


type Pair[K, V] = tuple[K, V]
T = 1
print(T, Pair, Box, first)
";

/// `from __future__ import annotations`: no annotation is evaluated where it stands.
const FUTURE_ANNOTATIONS: &str = "\
from __future__ import annotations


def make(value: Later) -> list[Later]:
    return [value]


class Later:
    link: Later
    count: int = 0
";

/// A source today's readers do not answer.
#[derive(Clone, Copy, Debug)]
enum Unanswered {
    /// A file with a syntax error: the model is built over the tree the parser recovers.
    SyntaxError(&'static str),
    /// One name bound this many times, a line each: more events of one name than today's chain limit of 2^20.
    OneNameBound(usize),
}

/// The rows of the cases today's readers do not answer: how many events the filler writes, and no cause.
const UNANSWERED: &[Case<Unanswered, Counted, Failure>] = &[
    Case {
        name: CaseName("a_file_with_a_syntax_error_has_its_events"),
        word: Word::Succeeds,
        input: || Unanswered::SyntaxError("if:\n    pass\nx = 1\nprint(x\ny = x +\n"),
        expect: Expect::Value(Counted { rows: 6, causes: Vec::new() }),
    },
    Case {
        name: CaseName("a_name_with_more_than_two_to_the_twenty_events_is_answered"),
        word: Word::Succeeds,
        input: || Unanswered::OneNameBound((1 << 20) + 1),
        expect: Expect::Value(Counted { rows: (1 << 20) + 1, causes: Vec::new() }),
    },
];

fn unanswered(input: Unanswered) -> Counted {
    let source = match input {
        Unanswered::SyntaxError(source) => source.to_owned(),
        Unanswered::OneNameBound(times) => "x = 0\n".repeat(times),
    };
    counted::<languages::BindingEventRow>(&source, |view, words, rows| binding_event_rows(view, words, rows))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
    run(EVERY, Word::Succeeds, |input| Ok(every(input)));
    run(UNANSWERED, Word::Succeeds, |input| Ok(unanswered(input)));
}
