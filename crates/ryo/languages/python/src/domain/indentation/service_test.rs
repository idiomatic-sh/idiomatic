//! Independent lexical goldens from Python Language Reference 2.1.8 and stdlib tokenize,
//! authored before the candidate scan. Byte endpoints are calculated from the source bytes.
use super::{
    service::{clear, prefix, regions},
    types::{IndentationIndex, IndentationState},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    syntax::{service::kind, types::Kind},
    tokens::types::Token,
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Position, Tree};
use std::cell::RefCell;
#[derive(Clone, Copy, Debug)]
enum Given {
    Source(&'static str),
    Reused,
}
/// What a row is given.
type Input = Given;
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shown {
    start: u32,
    end: u32,
    parent_start: u32,
    indentation: &'static str,
}
type Value = Option<&'static [Shown]>;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty"),
        word: Word::Succeeds,
        input: || Given::Source(""),
        expect: Expect::Value(Some(&[Shown {
            start: 0,
            end: 0,
            parent_start: 0,
            indentation: "",
        }])),
    },
    Case {
        name: CaseName("comments_only"),
        word: Word::Succeeds,
        input: || Given::Source("# note\n"),
        expect: Expect::Value(Some(&[Shown {
            start: 0,
            end: 7,
            parent_start: 0,
            indentation: "",
        }])),
    },
    Case {
        name: CaseName("balanced_indentation_survives_target_syntax_error"),
        word: Word::Succeeds,
        input: || Given::Source("value = ur\"text\" # note\nif ready:\n    work()\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 45,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 34,
                end: 45,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
    Case {
        name: CaseName("unclosed_delimiter_does_not_erase_complete_root_region"),
        word: Word::Succeeds,
        input: || Given::Source("x= (\n"),
        expect: Expect::Value(Some(&[Shown {
            start: 0,
            end: 5,
            parent_start: 0,
            indentation: "",
        }])),
    },
    Case {
        name: CaseName("unclosed_delimiter_retains_indentation_and_comment"),
        word: Word::Succeeds,
        input: || Given::Source("if ready:\n    call( # retained\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 31,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 10,
                end: 31,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
    Case {
        name: CaseName("eof_without_newline"),
        word: Word::Succeeds,
        input: || Given::Source("if yes:\n    pass"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 16,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 8,
                end: 16,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
    Case {
        name: CaseName("nested_same_offset_dedents"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n    if y:\n        pass\nz=0\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 33,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 6,
                end: 29,
                parent_start: 0,
                indentation: "    ",
            },
            Shown {
                start: 16,
                end: 29,
                parent_start: 6,
                indentation: "        ",
            },
        ])),
    },
    Case {
        name: CaseName("leading_and_trailing_comments"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n    # lead\n    pass\n    # trail\nx=0\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 42,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 17,
                end: 38,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
    Case {
        name: CaseName("tabs"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n\tpass\nx=0\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 16,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 6,
                end: 12,
                parent_start: 0,
                indentation: "\t",
            },
        ])),
    },
    Case {
        name: CaseName("formfeed"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n \u{c}    pass\nx=0\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 21,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 6,
                end: 17,
                parent_start: 0,
                indentation: " \u{c}    ",
            },
        ])),
    },
    Case {
        name: CaseName("decorator"),
        word: Word::Succeeds,
        input: || Given::Source("@decorator\ndef f():\n    pass\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 29,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 20,
                end: 29,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
    Case {
        name: CaseName("multiline_string"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n    text = '''a\n        b\n    c'''\nnext=0\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 48,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 6,
                end: 41,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
    Case {
        name: CaseName("unicode_before_indent"),
        word: Word::Succeeds,
        input: || Given::Source("name = 'é'\nif x:\n    pass\nnext=0\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 34,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 18,
                end: 27,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
    Case {
        name: CaseName("complete_tokens_preserve_indentation_after_syntax_error"),
        word: Word::Succeeds,
        input: || Given::Source("if:\n    pass\n"),
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 13,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 4,
                end: 13,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
    Case {
        name: CaseName("unbalanced_indentation_is_incomplete"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n  pass\n pass\n"),
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("retained_valid_invalid_empty_valid_models_do_not_leak"),
        word: Word::Succeeds,
        input: || Given::Reused,
        expect: Expect::Value(Some(&[
            Shown {
                start: 0,
                end: 16,
                parent_start: 0,
                indentation: "",
            },
            Shown {
                start: 8,
                end: 16,
                parent_start: 0,
                indentation: "    ",
            },
        ])),
    },
];
fn read(source: &'static str, cache: &mut RefCell<IndentationIndex>) -> Value {
    clear(cache.get_mut());
    assert_eq!(cache.borrow().state, IndentationState::Fresh);
    let file = parsed(source.as_bytes()).expect("recovered source");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let result = regions(&tree, cache);
    result.map(|index| -> &'static [Shown] {
        assert_eq!(index.state, IndentationState::Complete);
        let again = regions(&tree, cache).expect("cached complete read");
        assert_eq!(again.rows, index.rows);
        index
            .rows
            .iter()
            .map(|region| {
                if region.id != Position(0) {
                    assert_eq!(kind(&tree, region.id), Kind::Token(Token::Indent));
                }
                let parent = index
                    .rows
                    .iter()
                    .find(|parent| parent.id == region.parent)
                    .expect("parent region");
                let symbol = prefix(&tree, &mut symbols, region);
                let start = usize::try_from(region.span.start.0).expect("source start fits usize");
                let end = usize::try_from(region.indentation_end.0).expect("source end fits usize");
                let raw = source
                    .get(start..end)
                    .expect("indentation endpoints are UTF-8 boundaries");
                assert_eq!(
                    languages::text_of(&symbols, tree.source, symbol).map(|text| text.0),
                    Some(raw)
                );
                Shown {
                    start: region.span.start.0,
                    end: region.span.end.0,
                    parent_start: parent.span.start.0,
                    indentation: raw,
                }
            })
            .collect::<Vec<_>>()
            .leak()
    })
}
fn subject(input: Input) -> Value {
    let mut cache = RefCell::new(IndentationIndex::default());
    match input {
        Given::Source(source) => read(source, &mut cache),
        Given::Reused => {
            assert!(read("if yes:\n    pass", &mut cache).is_some());
            assert!(read("value = 'unterminated\n", &mut cache).is_none());
            assert_eq!(cache.borrow().state, IndentationState::Incomplete);
            assert_eq!(
                read("", &mut cache),
                Some::<&[Shown]>(&[Shown {
                    start: 0,
                    end: 0,
                    parent_start: 0,
                    indentation: ""
                }])
            );
            read("if yes:\n    pass", &mut cache)
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
