//! Independent lexical goldens from Python Language Reference 2.1.8 and stdlib tokenize,
//! authored before the candidate scan. Byte endpoints are calculated from the source bytes.
use super::{
    service::{clear, prefix, regions},
    types::{IndentationIndex, IndentationState},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    nodes::types::Node,
    syntax::{service::kind, types::Kind},
    tokens::types::Token,
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};
use std::cell::RefCell;
#[derive(Clone, Copy, Debug)]
enum Given {
    Source(&'static str),
    Reused,
}
/// What a row is given.
type Input = Given;
/// One region of the index: its span, its parent's start, the kind of the element it is anchored on, the text the
/// `prefix` symbol names and the indentation bytes of the source.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Shown {
    start: u32,
    end: u32,
    parent_start: u32,
    kind: Kind,
    prefix: Option<&'static str>,
    indentation: &'static str,
}
/// What a second read of the same tree answers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Reread {
    /// The cached rows again.
    Same,
    /// Other rows.
    Differs,
    /// No index.
    Refused,
    /// The first read answered no index, so none is reread.
    Skipped,
}
/// One read: the cache state after `clear`, after the read, the reread, and the regions.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Read {
    cleared: IndentationState,
    read: IndentationState,
    reread: Reread,
    regions: Option<&'static [Shown]>,
}
type Value = &'static [Read];
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("empty"),
        word: Word::Succeeds,
        input: || Given::Source(""),
        expect: Expect::Value(&[Read {
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
        }]),
    },
    Case {
        name: CaseName("comments_only"),
        word: Word::Succeeds,
        input: || Given::Source("# note\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Complete,
            reread: Reread::Same,
            regions: Some(&[Shown {
                start: 0,
                end: 7,
                parent_start: 0,
                kind: Kind::Node(Node::Module),
                prefix: Some(""),
                indentation: "",
            }]),
        }]),
    },
    Case {
        name: CaseName("eof_without_newline"),
        word: Word::Succeeds,
        input: || Given::Source("if yes:\n    pass"),
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
                    start: 8,
                    end: 16,
                    parent_start: 0,
                    kind: Kind::Token(Token::Indent),
                    prefix: Some("    "),
                    indentation: "    ",
                },
            ]),
        }]),
    },
    Case {
        name: CaseName("unbalanced_indentation_is_incomplete"),
        word: Word::Succeeds,
        input: || Given::Source("if x:\n  pass\n pass\n"),
        expect: Expect::Value(&[Read {
            cleared: IndentationState::Fresh,
            read: IndentationState::Incomplete,
            reread: Reread::Skipped,
            regions: None,
        }]),
    },
];
fn read(source: &'static str, cache: &mut RefCell<IndentationIndex>) -> Result<Read, Failure> {
    clear(cache.get_mut());
    let cleared = cache.borrow().state;
    let file = parsed(source.as_bytes()).map_err(drop)?;
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let Some(index) = regions(&tree, cache) else {
        return Ok(Read {
            cleared,
            read: cache.borrow().state,
            reread: Reread::Skipped,
            regions: None,
        });
    };
    let read = index.state;
    let reread = match regions(&tree, cache) {
        Some(again) if again.rows == index.rows => Reread::Same,
        Some(_) => Reread::Differs,
        None => Reread::Refused,
    };
    let shown = index
        .rows
        .iter()
        .map(|region| {
            let parent_start = index
                .rows
                .iter()
                .find(|parent| parent.id == region.parent)
                .map_or(u32::MAX, |parent| parent.span.start.0);
            let symbol = prefix(&tree, &mut symbols, region);
            let prefix = languages::text_of(&symbols, tree.source, symbol)
                .map(|text| &*Box::leak(text.0.to_owned().into_boxed_str()));
            let start = usize::try_from(region.span.start.0).unwrap_or(usize::MAX);
            let end = usize::try_from(region.indentation_end.0).unwrap_or(usize::MAX);
            Shown {
                start: region.span.start.0,
                end: region.span.end.0,
                parent_start,
                kind: kind(&tree, region.id),
                prefix,
                indentation: source.get(start..end).unwrap_or("<not a UTF-8 boundary>"),
            }
        })
        .collect::<Vec<_>>()
        .leak();
    Ok(Read {
        cleared,
        read,
        reread,
        regions: Some(shown),
    })
}
fn subject(input: Input) -> Result<Value, Failure> {
    let mut cache = RefCell::new(IndentationIndex::default());
    let reads = match input {
        Given::Source(source) => vec![read(source, &mut cache)?],
        Given::Reused => vec![
            read("if yes:\n    pass", &mut cache)?,
            read("value = 'unterminated\n", &mut cache)?,
            read("", &mut cache)?,
            read("if yes:\n    pass", &mut cache)?,
        ],
    };
    Ok(reads.leak())
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[cfg(test)]
mod layout_test;
#[cfg(test)]
mod recovery_test;
