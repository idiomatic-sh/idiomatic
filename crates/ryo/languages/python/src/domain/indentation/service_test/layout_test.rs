//! The layout cases of the parent test file: nesting, comments, tabs, form feeds, decorators, multi-line strings and non-ASCII text before an indent.

use super::{Given, Read, Reread, Shown};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::indentation::types::IndentationState;
use crate::domain::nodes::types::Node;
use crate::domain::syntax::types::Kind;
use crate::domain::tokens::types::Token;

/// What a row is given: the parent table's input.
type Input = super::Input;
/// What a row expects: the parent table's value.
type Value = super::Value;
/// What a refused row expects: the parent table's failure.
type Failure = super::Failure;

/// Runs one row through the parent table's subject.
fn subject(input: Input) -> Result<Value, Failure> {
    super::subject(input)
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("nested_same_offset_dedents"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n    if y:\n        pass\nz=0\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 33,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 6,
                    end: 29,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
                Shown {
                    start: 16,
                    end: 29,
                    parent_start: 6,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("        "),
                    indentation: "        ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("leading_and_trailing_comments"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n    # lead\n    pass\n    # trail\nx=0\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 42,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 17,
                    end: 38,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("tabs"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n\tpass\nx=0\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 16,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 6,
                    end: 12,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("\t"),
                    indentation: "\t",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("formfeed"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n \u{c}    pass\nx=0\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 21,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 6,
                    end: 17,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some(" \u{c}    "),
                    indentation: " \u{c}    ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("decorator"),
        word: Word::Succeeds,
        input: || Given::Source("@decorator\ndef f():\n    pass\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 29,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 20,
                    end: 29,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("multiline_string"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n    text = '''a\n        b\n    c'''\nnext=0\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 48,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 6,
                    end: 41,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("unicode_before_indent"),
        word: Word::Succeeds,
        input: || Given::Source("name = 'é'\nif x:\n    pass\nnext=0\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 34,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 18,
                    end: 27,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
            ]),
        }]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
