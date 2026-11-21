//! The cases of the `Graph(FixedPoints)` fill (`follow`), each run through `made` on a pack the contract's encoding
//! wrote: for each key, each pair of a source and a fixed point it reaches, the key first. A case says the table's rows,
//! sorted (their order is the fill's), and the fill's counts.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::records::types::EdgeColumns;
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{FillInput, GraphFill, MadeFill, MadeTable};
use crate::domain::made_tables::fixtures::packs::{
    FIRST, SECOND, of, pipeline_of, read, scan, table, word,
};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::steps::types::{ColumnList, MadeColumn, MadeTableId};
use std::borrow::Cow;

/// The edges a case follows.
#[derive(Clone, Copy, Debug)]
enum Followed {
    /// The contract's stated rows: key 7, edges (1, 2), (2, 3), (3, 3).
    Stated,
    /// No key, one graph of the file: a chain to a fixed point, an edge written twice, an edge that runs out, a ring
    /// of two that reaches no fixed point, and a source of two targets that reaches two fixed points.
    OneGraph,
    /// No edge.
    Empty,
}

type Input = Followed;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("each_source_answers_the_fixed_point_it_reaches_with_its_key"),
        word: Word::Succeeds,
        input: || Followed::Stated,
        expect: Expect::Value(Cow::Borrowed(
            "7 1 3; 7 2 3; 7 3 3 | read 18 given 12 made 3",
        )),
    },
    Case {
        name: CaseName("a_walk_that_ends_or_only_circles_has_no_row_and_two_targets_answer_both"),
        word: Word::Succeeds,
        input: || Followed::OneGraph,
        expect: Expect::Value(Cow::Borrowed(
            "1 3; 10 10; 2 3; 3 3; 9 10; 9 3 | read 56 given 36 made 6",
        )),
    },
    Case {
        name: CaseName("no_edge_gives_no_row"),
        word: Word::Succeeds,
        input: || Followed::Empty,
        expect: Expect::Value(Cow::Borrowed(" | read 0 given 0 made 0")),
    },
];

/// The relation's rows of a case: an edge's source and its destination.
fn told(followed: Followed) -> Vec<[u64; 2]> {
    match followed {
        Followed::Stated => vec![[1, 2], [2, 3], [3, 3]],
        Followed::OneGraph => vec![
            [1, 2],
            [2, 3],
            [3, 3],
            [2, 3],
            [4, 5],
            [6, 8],
            [8, 6],
            [9, 3],
            [9, 10],
            [10, 10],
        ],
        Followed::Empty => Vec::new(),
    }
}

/// The table of a case: the key when it has one, the source, the fixed point.
fn tables(followed: Followed) -> Vec<MadeTable> {
    let stages = vec![scan(0), read(FIRST, 0, 1), read(SECOND, 0, 2)];
    let (row, keys, source) = match followed {
        Followed::Stated | Followed::Empty => {
            (vec![word(7), of(1), of(2)], vec![MadeColumn(0)], 1_u16)
        }
        Followed::OneGraph => (vec![of(1), of(2)], Vec::new(), 0),
    };
    let width = keys.len().saturating_add(2);
    let columns = EdgeColumns {
        source: MadeColumn(source),
        destination: MadeColumn(source.saturating_add(1)),
    };
    vec![table(
        1,
        width,
        MadeFill::Graph(
            GraphFill::FixedPoints(columns),
            Box::new([FillInput {
                pipelines: Box::new([pipeline_of(stages, row)]),
                keys: ColumnList(keys.into_boxed_slice()),
                order: None,
            }]),
        ),
    )]
}

fn subject(followed: Input) -> Result<Value, Failure> {
    let mut world = world_with(
        tables(followed),
        Vec::new(),
        &told(followed),
        Telling::Keyed,
    )?;
    let (mut frame, looped) = framed(&mut world);
    let last = MadeTableId(0);
    let _now = made(&mut frame, looped, last)?;
    let seen = shown(frame.store, last);
    let mut rows: Vec<&str> = seen.split("; ").filter(|row| !row.is_empty()).collect();
    rows.sort_unstable();
    let work = table_of(frame.store, last)
        .map(|slot| slot.work)
        .unwrap_or_default();
    Ok(Cow::Owned(format!(
        "{} | read {} given {} made {}",
        rows.join("; "),
        work.read.0,
        work.given.0,
        work.made.0
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
