//! The `defines` item's cases, every row in full, and the collection's shared cases.

use super::super::fixtures::contract::check;
use super::item::DefinesItem;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::fixtures::reading::{SHOWN, reading};
use ryo::languages::contract::{Reading, want_file, want_name, want_node, want_span, want_unknown};

/// The source the item reads.
type Input = &'static str;
/// What the item writes for it: every row, every column shown, and why it reads the relation only in part.
type Value = Reading;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_binding_is_defined_in_its_scope"),
    word: Word::Succeeds,
    input: || "import a.b\nfrom m import *\ndef f(p, *args, **kw):\n    global g\n    g = 1\n    try:\n        pass\n    except E as e:\n        pass\nclass C[T]:\n    def m(self): pass\ntype Alias = int\n[(w := 1) for i in r]\nmatch v:\n    case [x, *rest] | {**rest}:\n        pass\n",
    expect: Expect::Value(Reading {
        rows: &[
            &[
                want_node(4),
                want_name("import"),
                want_name("import"),
                want_name("a"),
                want_node(0),
                want_file(SHOWN),
                want_span((1, 8), (1, 9)),
            ],
            &[
                want_node(18),
                want_name("import"),
                want_name("from-import"),
                want_unknown("wildcard-import"),
                want_node(0),
                want_file(SHOWN),
                want_span((2, 15), (2, 16)),
            ],
            &[
                want_node(22),
                want_name("function"),
                want_name("function"),
                want_name("f"),
                want_node(0),
                want_file(SHOWN),
                want_span((3, 5), (3, 6)),
            ],
            &[
                want_node(30),
                want_name("parameter"),
                want_name("positional-or-keyword"),
                want_name("p"),
                want_node(22),
                want_file(SHOWN),
                want_span((3, 7), (3, 8)),
            ],
            &[
                want_node(35),
                want_name("parameter"),
                want_name("var-positional"),
                want_name("args"),
                want_node(22),
                want_file(SHOWN),
                want_span((3, 11), (3, 15)),
            ],
            &[
                want_node(41),
                want_name("parameter"),
                want_name("var-keyword"),
                want_name("kw"),
                want_node(22),
                want_file(SHOWN),
                want_span((3, 19), (3, 21)),
            ],
            &[
                want_node(57),
                want_name("variable"),
                want_name("variable"),
                want_name("g"),
                want_node(0),
                want_file(SHOWN),
                want_span((5, 5), (5, 6)),
            ],
            &[
                want_node(76),
                want_name("variable"),
                want_name("exception"),
                want_name("e"),
                want_node(22),
                want_file(SHOWN),
                want_span((8, 17), (8, 18)),
            ],
            &[
                want_node(94),
                want_name("class"),
                want_name("class"),
                want_name("C"),
                want_node(0),
                want_file(SHOWN),
                want_span((10, 7), (10, 8)),
            ],
            &[
                want_node(101),
                want_name("type-parameter"),
                want_name("type-var"),
                want_name("T"),
                want_node(94),
                want_file(SHOWN),
                want_span((10, 9), (10, 10)),
            ],
            &[
                want_node(108),
                want_name("method"),
                want_name("method"),
                want_name("m"),
                want_node(94),
                want_file(SHOWN),
                want_span((11, 9), (11, 10)),
            ],
            &[
                want_node(116),
                want_name("parameter"),
                want_name("positional-or-keyword"),
                want_name("self"),
                want_node(108),
                want_file(SHOWN),
                want_span((11, 11), (11, 15)),
            ],
            &[
                want_node(129),
                want_name("type-alias"),
                want_name("type-alias"),
                want_name("Alias"),
                want_node(0),
                want_file(SHOWN),
                want_span((12, 6), (12, 11)),
            ],
            &[
                want_node(142),
                want_name("variable"),
                want_name("variable"),
                want_name("w"),
                want_node(0),
                want_file(SHOWN),
                want_span((13, 3), (13, 4)),
            ],
            &[
                want_node(154),
                want_name("variable"),
                want_name("variable"),
                want_name("i"),
                want_node(138),
                want_file(SHOWN),
                want_span((13, 15), (13, 16)),
            ],
            &[
                want_node(177),
                want_name("variable"),
                want_name("pattern"),
                want_name("x"),
                want_node(0),
                want_file(SHOWN),
                want_span((15, 11), (15, 12)),
            ],
            &[
                want_node(182),
                want_name("variable"),
                want_name("pattern"),
                want_name("rest"),
                want_node(0),
                want_file(SHOWN),
                want_span((15, 15), (15, 19)),
            ],
            &[
                want_node(190),
                want_name("variable"),
                want_name("pattern"),
                want_name("rest"),
                want_node(0),
                want_file(SHOWN),
                want_span((15, 26), (15, 30)),
            ],
        ],
        causes: &[],
    }),
}];

fn subject(source: Input) -> Value {
    check::<DefinesItem>();
    reading::<DefinesItem>(source)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
