//! URL semantics apply only where HTML defines a URL-valued attribute, and its target is resolved to a path of the run.
use super::ReferenceItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};
type Input = &'static str;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("unrelated_attributes_with_url_like_names_are_not_references"),
        word: Word::Succeeds,
        input: || "<!doctype html><p data='not-a-url' action='not-an-action'><div src='not-a-source'></div>",
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName(
            "a_url_is_read_without_the_white_space_around_it_and_resolved_to_a_path_of_the_run",
        ),
        word: Word::Succeeds,
        input: || "<!doctype html><a href=\" sub/../page.html#x \">p</a>",
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(17),
                want_node(0),
                want_name("page.html#x"),
                want_file(SHOWN),
                want_span((1, 19), (1, 46)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_relative_url_is_resolved_against_the_base_the_document_names"),
        word: Word::Succeeds,
        input: || "<!doctype html><base href=\"/assets/\"><a href=\"x.html\">p</a>",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(14),
                    want_node(0),
                    want_name("assets"),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 37)),
                ],
                &[
                    want_node(23),
                    want_node(0),
                    want_name("assets/x.html"),
                    want_file(SHOWN),
                    want_span((1, 41), (1, 54)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_fragment_alone_names_the_file_whatever_the_base"),
        word: Word::Succeeds,
        input: || "<!doctype html><base href=\"/assets/\"><a href=\"#top\">p</a>",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(14),
                    want_node(0),
                    want_name("assets"),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 37)),
                ],
                &[
                    want_node(23),
                    want_node(0),
                    want_name("item.html#top"),
                    want_file(SHOWN),
                    want_span((1, 41), (1, 52)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_relative_url_under_a_base_outside_the_run_stays_as_written"),
        word: Word::Succeeds,
        input: || "<!doctype html><base href=\"https://cdn.example/\"><a href=\"x.html\">p</a>",
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(14),
                    want_node(0),
                    want_name("https://cdn.example/"),
                    want_file(SHOWN),
                    want_span((1, 22), (1, 49)),
                ],
                &[
                    want_node(23),
                    want_node(0),
                    want_name("x.html"),
                    want_file(SHOWN),
                    want_span((1, 53), (1, 66)),
                ],
            ],
            causes: &[],
        }),
    },
];
#[test]
fn succeeds() {
    super::super::super::fixtures::contract::check::<ReferenceItem>();
    run(CASES, Word::Succeeds, |source| {
        Ok(reading::<ReferenceItem>(source))
    });
}
