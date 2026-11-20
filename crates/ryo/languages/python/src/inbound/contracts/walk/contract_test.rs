//! The `walk` contract's cases: over every Python source of the language's fixtures (514 files, the parser's suite among them), the walk writes the tree today's reading writes
//! (every element, under the same parent, at the same place; in the same field wherever the parser accepts the file) and its table keeps the laws of the engine's node
//! table; a file that is not Python is not walked.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::labels::types::Field;
use crate::domain::syntax::service::{kind_name, kind_of, kinds};
use crate::fixtures::table_laws::broken_laws;
use crate::fixtures::today::{Read, read, read_keys, read_placements};
use crate::fixtures::walking::{Placement, walked, walked_keys, walked_placements};
use crate::outbound::syntax::fixtures::library::fixture_sources;
use ryo::languages::{self, NodeTable, Offset, Walk, WalkError, node_table};

/// What a case asks of the walk.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// What differs between the walk's tree and today's, over every fixture source.
    Trees,
    /// The laws of the node table the walked tables break, over every fixture source.
    Laws,
    /// Whether the walk's kind count is the count of the language's kinds.
    Kinds,
    /// What the walk answers for a file at this path.
    Path(&'static str),
}

type Input = Asked;
/// What was found, one line for each finding; empty when nothing differs and no law is broken.
type Value = String;
type Failure = WalkError;

/// The rows of the contract's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_walk_writes_the_tree_today_s_reading_writes"),
        word: Word::Succeeds,
        input: || Asked::Trees,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_walked_table_keeps_the_laws_of_the_node_table"),
        word: Word::Succeeds,
        input: || Asked::Laws,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("the_kind_count_is_the_count_of_the_language_s_kinds"),
        word: Word::Succeeds,
        input: || Asked::Kinds,
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_python_file_is_walked"),
        word: Word::Succeeds,
        input: || Asked::Path("package/module.py"),
        expect: Expect::Value(String::new()),
    },
    Case {
        name: CaseName("a_file_that_is_not_python_is_not_walked"),
        word: Word::Refuses,
        input: || Asked::Path("notes.txt"),
        expect: Expect::Error(WalkError::Unsupported),
    },
];

/// One placement, shown by the names of its kinds and of its field.
fn shown(placement: &Placement) -> String {
    let field: &'static str = Field::from_repr(placement.field).unwrap_or_default().into();
    format!(
        "{} {}..{} #{} under {} {}..{} at {} in {field}",
        kind_name(kind_of(placement.element.kind)).0,
        placement.element.start.0,
        placement.element.end.0,
        placement.element.nth,
        kind_name(kind_of(placement.parent.kind)).0,
        placement.parent.start.0,
        placement.parent.end.0,
        placement.ordinal,
    )
}

/// The first placements one tree has and the other lacks.
fn lacking(folder: &str, side: &str, have: &[Placement], other: &[Placement]) -> String {
    have.iter()
        .filter(|each| other.binary_search(each).is_err())
        .take(3)
        .map(|each| format!("{folder}: only {side}: {}\n", shown(each)))
        .collect()
}

/// What differs between the two trees of one source.
fn tree_differences(folder: &str, bytes: &[u8]) -> Result<String, WalkError> {
    let linked = crate::linked();
    let mut today = Read::default();
    read(
        linked.parse_engine().as_ref(),
        "input.py",
        bytes,
        &[],
        &mut today,
    )
    .map_err(|_refused| WalkError::Library)?;
    let read_keys = read_keys(&today.elements);
    let old = read_placements(&today.elements, &read_keys);
    let walk = crate::walk();
    let (mut table, mut scratch) = (NodeTable::default(), Default::default());
    let (mut new, mut accepted) = (Vec::new(), false);
    walked(
        &walk,
        "input.py",
        bytes,
        &[],
        &mut table,
        &mut scratch,
        &mut |file| {
            new = walked_placements(file.table, &walked_keys(file.table));
            accepted =
                node_table::acceptance_of(file.table) == languages::SyntaxAcceptance::Accepted;
        },
    )?;
    let mut old = old;
    if !accepted {
        // In a file the parser refuses, today's reading tells the field of a recovered node from where it stands, and the walk
        // from the parser's own typed node: the trees are held to the same elements, parents and places, not to the same fields.
        for each in old.iter_mut().chain(new.iter_mut()) {
            each.field = 0;
        }
        old.sort_unstable();
        new.sort_unstable();
    }
    Ok(format!(
        "{}{}",
        lacking(folder, "today", &old, &new),
        lacking(folder, "walked", &new, &old)
    ))
}

/// The laws the walked table of one source breaks.
fn law_differences<W: Walk>(walk: &W, folder: &str, bytes: &[u8]) -> Result<String, WalkError> {
    let (mut table, mut scratch) = (NodeTable::default(), Default::default());
    let mut found = String::new();
    let length = Offset(u32::try_from(bytes.len()).unwrap_or(u32::MAX));
    walked(
        walk,
        "input.py",
        bytes,
        &[],
        &mut table,
        &mut scratch,
        &mut |file| {
            found = broken_laws(file.table, W::KIND_COUNT, length)
                .iter()
                .take(3)
                .map(|law| format!("{folder}: {law}\n"))
                .collect();
        },
    )?;
    Ok(found)
}

/// The walk's kind count against the language's kinds.
fn kind_difference<W: Walk>(_walk: &W) -> String {
    if W::KIND_COUNT == languages::Count(kinds().count()) {
        String::new()
    } else {
        format!(
            "the walk counts {} kinds, the language has {}",
            W::KIND_COUNT.0,
            kinds().count()
        )
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    match asked {
        Asked::Trees => fixture_sources()
            .map_err(|_unread| WalkError::Library)?
            .iter()
            .map(|(path, bytes)| tree_differences(path, bytes))
            .collect(),
        Asked::Laws => fixture_sources()
            .map_err(|_unread| WalkError::Library)?
            .iter()
            .map(|(path, bytes)| law_differences(&crate::walk(), path, bytes))
            .collect(),
        Asked::Kinds => Ok(kind_difference(&crate::walk())),
        Asked::Path(path) => {
            let (mut table, mut scratch) = (NodeTable::default(), Default::default());
            let mut visits = 0_u32;
            walked(
                &crate::walk(),
                path,
                b"x = 1\n",
                &[],
                &mut table,
                &mut scratch,
                &mut |_file| {
                    visits += 1;
                },
            )?;
            Ok(if visits == 1 {
                String::new()
            } else {
                format!("the file was lent {visits} times")
            })
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod rows_test;
