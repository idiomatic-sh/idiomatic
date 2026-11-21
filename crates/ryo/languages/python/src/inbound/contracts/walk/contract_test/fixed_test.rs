//! The `walk` contract's cases of the fixed names: a fixed word is the same cell in every file of the run, and it is the word of
//! the handle the language's vocabulary gives its name; over every Python source of the language's fixtures, every text a
//! filler writes as a fixed word is a fixed name the vocabulary states, so none is interned for a file; the vocabulary states
//! each fixed name once and none that is empty.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::file_view::types::PythonWalk;
use crate::domain::token_links::constants::NEXT_TOKEN;
use crate::fixtures::walking::{walk_words, walked};
use crate::inbound::contracts::walk::contract::{PythonWalkScratch, WalkPorts};
use crate::outbound::syntax::factory::ruff_python_parser;
use crate::outbound::syntax::fixtures::library::fixture_sources;
use crate::settings::PythonParserSettings;
use ryo::languages::{
    self, FactAnswer, FilledColumns, FixedWord, NodeTable, Walk, WalkError, Wanted, facts,
};
use std::sync::Arc;

/// What a case asks of the fixed names.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The kind of the first link of each of two files of one run, as the cell and as its text, and the word of the handle
    /// the vocabulary gives `next-token`.
    SameCell,
    /// How many fixed names the fillers wrote by a handle over every fixture source, and the texts they wrote as fixed words
    /// that the vocabulary does not state.
    Declared,
    /// How many fixed names the vocabulary states, the names it states at more than one place, and how many are empty.
    Stated,
}

type Input = Asked;
/// What was found, as one text.
type Value = &'static str;
type Failure = WalkError;

/// The rows of the contract's cases of the fixed names.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_fixed_word_is_the_same_cell_in_every_file_of_the_run"),
        word: Word::Succeeds,
        input: || Asked::SameCell,
        expect: Expect::Value("word 1 `next-token`, word 1 `next-token`, handle 0 is word 1"),
    },
    Case {
        name: CaseName("every_fixed_word_a_filler_writes_is_a_fixed_name_of_the_run"),
        word: Word::Succeeds,
        input: || Asked::Declared,
        expect: Expect::Value("261 fixed names written by handle; not fixed names: []"),
    },
    Case {
        name: CaseName("each_fixed_name_is_stated_once_and_none_is_empty"),
        word: Word::Succeeds,
        input: || Asked::Stated,
        expect: Expect::Value("300 fixed names; stated twice: []; empty: 0"),
    },
];

/// The fixed names the walk's vocabulary states: how many, the names stated at more than one place, and how many are the empty
/// text. A fixed name's word is its place, so a name has one place, and the empty text is word 0 of every run and no name's.
fn stated<W: Walk>(_walk: &W) -> String {
    let names = W::vocabulary().fixed.0;
    let mut sorted: Vec<&str> = names.iter().map(|name| name.0).collect();
    sorted.sort_unstable();
    let mut twice: Vec<&str> = sorted
        .windows(2)
        .filter_map(|pair| {
            (pair.first() == pair.get(1))
                .then(|| pair.first().copied())
                .flatten()
        })
        .collect();
    twice.dedup();
    let empty = names.iter().filter(|name| name.0.is_empty()).count();
    format!(
        "{} fixed names; stated twice: {twice:?}; empty: {empty}",
        names.len()
    )
}

/// The language's walk, as the crate links it, by its own type: its worker's buffers can be read.
fn own_walk() -> PythonWalk<WalkPorts> {
    let settings = PythonParserSettings::default();
    PythonWalk {
        ports: WalkPorts {
            settings,
            syntax: Arc::new(ruff_python_parser(settings)),
        },
    }
}

/// The kind of the first link of each of two files walked by one worker into one set of words, forgotten between the files:
/// `word N \`text\`` for each, then the word of the handle of `next-token`.
fn same_cell<W: Walk>(walk: &W) -> Result<String, WalkError> {
    let (mut table, mut scratch) = (NodeTable::default(), W::Scratch::default());
    let mut words = walk_words(walk);
    let mut shown = Vec::new();
    for source in ["a = 1\n", "import os\nprint(os.name, [1, 2])\n"] {
        facts::clear_words(&mut words);
        // A word of the file's own, so that a fixed word interned for a file would be numbered after it.
        let _own = facts::intern(&mut words, languages::NameText(source));
        walked(
            walk,
            "input.py",
            source.as_bytes(),
            &[],
            &mut table,
            &mut scratch,
            &mut |file| {
                let mut links = FilledColumns::default();
                let answer = file.facts.fill(
                    languages::Relation::TokenLink,
                    Wanted::Whole,
                    &mut words,
                    &mut links,
                );
                let kind = links.0.get(2).and_then(|cells| cells.0.first().copied());
                shown.push(match (answer, kind) {
                    (FactAnswer::Answered(_), Some(cell)) => {
                        format!("word {} `{}`", cell.0, facts::text(&words, cell).0)
                    }
                    (answer, _) => format!("no link: {answer:?}"),
                });
            },
        )?;
    }
    let place = W::vocabulary()
        .fixed
        .0
        .iter()
        .position(|name| name.0 == NEXT_TOKEN);
    let handle = place.and_then(|at| u32::try_from(at).ok()).map(FixedWord);
    shown.push(handle.map_or_else(
        || "no handle".to_owned(),
        |handle| {
            format!(
                "handle {} is word {}",
                handle.0,
                facts::fixed(&words, handle).0
            )
        },
    ));
    Ok(shown.join(", "))
}

/// Every relation filled whole for every fixture source by one worker: how many fixed names it wrote by a handle, and the
/// texts it wrote as fixed words that are no fixed name of the run.
fn declared() -> Result<String, WalkError> {
    let walk = own_walk();
    let (mut table, mut scratch) = (NodeTable::default(), PythonWalkScratch::default());
    let mut words = walk_words(&walk);
    for (_path, bytes) in fixture_sources().map_err(|_unread| WalkError::Library)? {
        facts::clear_words(&mut words);
        walked(
            &walk,
            "input.py",
            &bytes,
            languages::all(),
            &mut table,
            &mut scratch,
            &mut |file| {
                for relation in languages::all() {
                    let _answer = file.facts.fill(
                        *relation,
                        Wanted::Whole,
                        &mut words,
                        &mut FilledColumns::default(),
                    );
                }
            },
        )?;
    }
    let held = &scratch.file.model.words.get_mut().fixed.held;
    let mut undeclared: Vec<&str> = held
        .iter()
        .filter(|(_, handle)| handle.is_none())
        .map(|(text, _)| text.0.0)
        .collect();
    undeclared.sort_unstable();
    // One constant may stand at more than one place of the program: the handles are counted, each once.
    let mut handles: Vec<FixedWord> = held.values().filter_map(|handle| *handle).collect();
    handles.sort_unstable();
    handles.dedup();
    Ok(format!(
        "{} fixed names written by handle; not fixed names: {undeclared:?}",
        handles.len()
    ))
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let found = match asked {
        Asked::SameCell => same_cell(&own_walk()),
        Asked::Declared => declared(),
        Asked::Stated => Ok(stated(&own_walk())),
    }?;
    Ok(Box::leak(found.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
