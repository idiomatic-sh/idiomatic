//! The recovery cases of the parent test file: indentation kept through syntax errors, and a reused cache that leaks nothing between models.

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
        name: CaseName("balanced_indentation_survives_target_syntax_error"),
        word: Word::Succeeds,
        input: || Given::Source("value = ur\"text\" # note\nif ready:\n    work()\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 45,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 34,
                    end: 45,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("unclosed_delimiter_does_not_erase_complete_root_region"),
        word: Word::Succeeds,
        input: || Given::Source("x= (\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[Shown {
                start: 0,
                end: 5,
                parent_start: 0,
                kind: Kind::Node(Node::Module),
                prefix: Some(""),
                indentation: "",
            }]),
        }]),
    },
    Case {
        name: CaseName("unclosed_delimiter_retains_indentation_and_comment"),
        word: Word::Succeeds,
        input: || Given::Source("if ready:\n    call( # retained\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 31,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 10,
                    end: 31,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("complete_tokens_preserve_indentation_after_syntax_error"),
        word: Word::Succeeds,
        input: || Given::Source("if:\n    pass\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[
                Shown {
                    start: 0,
                    end: 13,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                },
                Shown {
                    start: 4,
                    end: 13,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("retained_valid_invalid_empty_valid_models_do_not_leak"),
        word: Word::Succeeds,
        input: || Given::Reused,
        expect: Expect::Value(&[
            Read {
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
                        start: 8,
                        end: 16,
                        parent_start: 0,
                        kind: Kind::Token(Token::Indent),
                        prefix: Some("    "),
                        indentation: "    ",
                    },
                ]),
            },
            Read {
                cleared: IndentationState::Fresh,
                read: IndentationState::Incomplete,
                reread: Reread::Skipped,
                regions: None,
            },
            Read {
                cleared: IndentationState::Fresh,
                read: IndentationState::Complete,
                reread: Reread::Same,
                regions: Some(&[Shown {
                    start: 0,
                    end: 0,
                    parent_start: 0,
                    kind: Kind::Node(Node::Module),
                    prefix: Some(""),
                    indentation: "",
                }]),
            },
            Read {
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
                        start: 8,
                        end: 16,
                        parent_start: 0,
                        kind: Kind::Token(Token::Indent),
                        prefix: Some("    "),
                        indentation: "    ",
                    },
                ]),
            },
        ]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
