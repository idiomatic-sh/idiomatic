//! The `inherits` item's cases, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::InheritsItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every cell of every row, and the causes for which it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_supertrait_and_an_implemented_trait_are_inherited"),
        word: Word::Succeeds,
        input: || "trait Parent {} trait Child: Parent {} struct Service; impl Parent for Service {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(20),
                    want_node(11),
                    want_name("Parent"),
                    want_file(SHOWN),
                    want_span((1, 30), (1, 36)),
                ],
                &[
                    want_node(40),
                    want_node(30),
                    want_name("Parent"),
                    want_file(SHOWN),
                    want_span((1, 61), (1, 67)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_implementation_before_the_declaration_it_names_is_linked"),
        word: Word::Succeeds,
        input: || "impl Parent for Service {}\ntrait Parent {}\nstruct Service;",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_node(32),
                want_name("Parent"),
                want_file(SHOWN),
                want_span((1, 6), (1, 12)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_type_declared_twice_is_inherited_through_the_implementation"),
        word: Word::Succeeds,
        input: || "trait Parent {}\nstruct Service;\nmod inner { struct Service; }\nimpl Parent for Service {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(39),
                want_node(36),
                want_name("Parent"),
                want_file(SHOWN),
                want_span((4, 6), (4, 12)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_enum_is_linked_like_a_struct"),
        word: Word::Succeeds,
        input: || "trait Parent {}\nenum Mode { On }\nimpl Parent for Mode {}",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(29),
                want_node(11),
                want_name("Parent"),
                want_file(SHOWN),
                want_span((3, 6), (3, 12)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_qualified_implementation_is_inherited_through_the_implementation"),
        word: Word::Succeeds,
        input: || "trait Base {}\ntrait Contract: Base + Clone {}\nimpl Contract for module::Service {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(20),
                    want_node(11),
                    want_name("Base"),
                    want_file(SHOWN),
                    want_span((2, 17), (2, 21)),
                ],
                &[
                    want_node(29),
                    want_node(11),
                    want_name("Clone"),
                    want_file(SHOWN),
                    want_span((2, 24), (2, 29)),
                ],
                &[
                    want_node(42),
                    want_node(39),
                    want_name("Contract"),
                    want_file(SHOWN),
                    want_span((3, 6), (3, 14)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("every_implementation_of_a_trait_has_a_row_whatever_it_is_for"),
        word: Word::Succeeds,
        input: || "trait Caches {}\nstruct Map;\nimpl<T> Caches for Option<T> {}\nimpl Caches for Map {}\nimpl Caches for [u8] {}\nimpl<T: Clone> Caches for &T {}\nimpl Caches for other::Remote {}",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(27),
                    want_node(18),
                    want_name("Caches"),
                    want_file(SHOWN),
                    want_span((3, 9), (3, 15)),
                ],
                &[
                    want_node(57),
                    want_node(11),
                    want_name("Caches"),
                    want_file(SHOWN),
                    want_span((4, 6), (4, 12)),
                ],
                &[
                    want_node(78),
                    want_node(75),
                    want_name("Caches"),
                    want_file(SHOWN),
                    want_span((5, 6), (5, 12)),
                ],
                &[
                    want_node(117),
                    want_node(99),
                    want_name("Caches"),
                    want_file(SHOWN),
                    want_span((6, 16), (6, 22)),
                ],
                &[
                    want_node(140),
                    want_node(137),
                    want_name("Caches"),
                    want_file(SHOWN),
                    want_span((7, 6), (7, 12)),
                ],
            ],
            causes: &[],
        }),
    },
];

fn subject(source: Input) -> Value {
    check::<InheritsItem>();
    reading::<InheritsItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
