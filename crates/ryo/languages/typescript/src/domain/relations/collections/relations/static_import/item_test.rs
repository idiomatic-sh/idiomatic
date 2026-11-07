//! Cases for TypeScript `static_import` rows, checked against the shared reader contract.
use super::super::fixtures::contract::check;
use super::item::StaticImportItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::reading_at;
use crate::fixtures::workspaces::{APP_PATH, APP_SOURCE, CARD_PATH, CARD_SOURCE};
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
        name: CaseName("a_line_comment_ends_at_its_block_not_at_the_next_one"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.vue",
            text: "<script lang=\"ts\">// c</script>\n<script>import a from 'x';</script>\n",
        },
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(4),
                want_node(0),
                want_name("a"),
                want_name("x"),
                want_file("item.vue"),
                want_span((2, 9), (2, 27)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("import_local_binding"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.ts",
            text: "import {a as b} from \"pkg\";",
        },
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_node(0),
                want_name("b"),
                want_name("pkg"),
                want_file("item.ts"),
                want_span((1, 1), (1, 28)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("an_empty_file_has_no_row"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.ts",
            text: "",
        },
        expect: Expect::Value(Reading {
            rows: &[],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_typescript_script_block_gives_its_import_at_its_place_in_the_file"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.vue",
            text: "<template><p>hi</p></template>\n<script lang=\"ts\">\nimport a from 'x';\n</script>\n",
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
        name: CaseName("a_script_block_beside_a_typescript_block_is_read_too"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.svelte",
            text: "<script>\nimport a from 'x';\n</script>\n<p>é</p>\n<script lang=\"ts\">\nimport b from 'y';\n</script>\n",
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
        name: CaseName("monorepo_tsx_file_gives_its_import"),
        word: Word::Succeeds,
        input: || Given {
            path: APP_PATH,
            text: APP_SOURCE,
        },
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(1),
                want_node(0),
                want_name("Button"),
                want_name("ui"),
                want_file("packages/app/src/App.tsx"),
                want_span((1, 1), (1, 29)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("monorepo_component_file_gives_its_import_at_its_place_in_the_file"),
        word: Word::Succeeds,
        input: || Given {
            path: CARD_PATH,
            text: CARD_SOURCE,
        },
        expect: Expect::Value(Reading {
            rows: &[&[
                want_node(2),
                want_node(0),
                want_name("ref"),
                want_name("vue"),
                want_file("packages/ui/src/Card.vue"),
                want_span((3, 1), (3, 27)),
            ]],
            causes: &[],
        }),
    },
    Case {
        name: CaseName("a_typed_file_with_no_import_has_no_row"),
        word: Word::Succeeds,
        input: || Given {
            path: "item.vue",
            text: "<script lang=\"ts\">\nconst a: number = 1;\n</script>\n",
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
