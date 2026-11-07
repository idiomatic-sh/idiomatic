//! `static_import` reader cases and the collection contract.
use super::super::fixtures::contract::check;
use super::item::StaticImportItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading_at;
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span};

/// A file to read: its path, and its text.
#[derive(Clone, Copy, Debug)]
struct Given {
    path: &'static str,
    text: &'static str,
}
type Input = Given;
type Value = Reading;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty_source_has_no_static_import"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.js",
            text: "",
        },
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_script_block_gives_its_import_at_its_place_in_the_file"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.vue",
            text: "<template><p>hi</p></template>\n<script>\nimport a from 'x';\n</script>\n",
        },
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(0),
                want_name("a"),
                want_name("x"),
                want_file("item.vue"),
                want_span((3, 1), (3, 19)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("two_script_blocks_give_their_imports_in_one_file"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.svelte",
            text: "<script context=\"module\">\nimport a from 'x';\n</script>\n<p>é</p>\n<script>\nimport b from 'y';\n</script>\n",
        },
        expect: Expect::Value(Reading {
            rows: &[
                &[
                    want_node(2),
                    want_node(0),
                    want_name("a"),
                    want_name("x"),
                    want_file("item.svelte"),
                    want_span((2, 1), (2, 19)),
                ],
                &[
                    want_node(15),
                    want_node(0),
                    want_name("b"),
                    want_name("y"),
                    want_file("item.svelte"),
                    want_span((6, 1), (6, 19)),
                ],
            ],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_line_comment_ends_at_its_block_not_at_the_next_one"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.svelte",
            text: "<script context=\"module\">// c</script>\n<script>import a from 'x';</script>\n",
        },
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_node(0),
                want_name("a"),
                want_name("x"),
                want_file("item.svelte"),
                want_span((2, 9), (2, 27)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_file_with_no_script_has_no_static_import"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.vue",
            text: "<template><p>hi</p></template>\n",
        },
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
];
fn subject(given: Input) -> Value {
    check::<StaticImportItem>();
    reading_at::<StaticImportItem>(given.path, given.text)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |given| Ok(subject(given)));
}
