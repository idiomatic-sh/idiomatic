//! The cases of a pipeline's stages over the file's nodes, each run through `made` on a pack the contract's encoding
//! wrote: the walks, a node row's cells, and the file's own tree. A case says its table's rows and the fill's work.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::{MadeFill, MadeTable};
use crate::domain::made_tables::fixtures::packs::{
    definition, keep_equal, node_read, of, pipeline, rows, scan_nodes, table, walk,
};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::sources::service::{made, table_of};
use crate::domain::steps::stages::types::{FillStage, TreeWalk};
use crate::domain::steps::types::{AxisDirection, MadeTableId, StepOperand};
use ryo_language::domain::node_table::types::FieldCode;
use ryo_relations::domain::relations::types::{NodeColumn, Relation};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use std::borrow::Cow;

/// The walk or the node read a case runs.
#[derive(Clone, Copy, Debug)]
enum Staged {
    /// The children of the root.
    WalkChildren,
    /// The children of the root under one field.
    WalkField,
    /// The descendants of the root.
    WalkDescendants,
    /// The ancestors of each token.
    WalkAncestors,
    /// The next sibling of each child of the root.
    WalkNext,
    /// The cells of a node row.
    ReadNode,
    /// A scan of the file's own tree.
    ScanOwn,
}

type Input = Staged;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The relation's rows.
const ROWS: [[u64; 2]; 5] = [[1, 10], [2, 20], [1, 30], [3, 10], [1, 40]];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_walk_gives_the_children_of_its_start"),
        word: Word::Succeeds,
        input: || Staged::WalkChildren,
        expect: Expect::Value(Cow::Borrowed(
            "30064771073 0; 30064771074 24; 30064771075 0 | read 31 given 23 made 3",
        )),
    },
    Case {
        name: CaseName("a_walk_of_one_field_gives_that_fields_children"),
        word: Word::Succeeds,
        input: || Staged::WalkField,
        expect: Expect::Value(Cow::Borrowed("30064771074 3 | read 27 given 19 made 1")),
    },
    Case {
        name: CaseName("a_walk_down_gives_each_descendant_node_in_order"),
        word: Word::Succeeds,
        input: || Staged::WalkDescendants,
        expect: Expect::Value(Cow::Borrowed(
            "30064771073 12; 30064771074 12; 30064771075 12 | read 31 given 23 made 3",
        )),
    },
    Case {
        name: CaseName("a_walk_up_gives_each_ancestor_nearest_first"),
        word: Word::Succeeds,
        input: || Staged::WalkAncestors,
        expect: Expect::Value(Cow::Borrowed(
            "30064771076 30064771073; 30064771076 30064771072; 30064771077 30064771074; 30064771077 30064771072; 30064771078 30064771075; 30064771078 30064771072 | read 33 given 25 made 6",
        )),
    },
    Case {
        name: CaseName("a_walk_to_the_next_sibling_gives_one_or_none"),
        word: Word::Succeeds,
        input: || Staged::WalkNext,
        expect: Expect::Value(Cow::Borrowed(
            "30064771073 30064771074; 30064771074 30064771075 | read 30 given 22 made 2",
        )),
    },
    Case {
        name: CaseName("a_read_of_a_node_row_gives_its_kind_field_and_text_as_words"),
        word: Word::Succeeds,
        input: || Staged::ReadNode,
        expect: Expect::Value(Cow::Borrowed(
            "30064771076 30064771073 0 0 0 2 2 7 1; 30064771077 30064771074 0 0 3 5 12884901893 7 2; 30064771078 30064771075 0 0 6 8 25769803784 7 3 | read 51 given 43 made 3",
        )),
    },
    Case {
        name: CaseName("a_scan_of_the_files_own_tree_leaves_embedded_elements_out"),
        word: Word::Succeeds,
        input: || Staged::ScanOwn,
        expect: Expect::Value(Cow::Borrowed(
            "own: 30064771072 11; 30064771073 12; 30064771074 12; 30064771075 12; 30064771076 13; 30064771077 13; 30064771078 13 | all: 30064771072 11; 30064771073 12; 30064771074 12; 30064771075 12; 30064771076 13; 30064771077 13; 30064771078 13; 30064771079 12 | read 24 given 16 made 8",
        )),
    },
];

/// The tables of a case; the last is the one made.
fn tables(staged: Staged) -> Vec<MadeTable> {
    let tree = Relation::SemanticSyntax;
    let root = |rest: Vec<FillStage>, row: [StepOperand; 2]| {
        let mut all = Vec::from(scan_nodes(tree));
        all.push(keep_equal(1, 11));
        all.extend(rest);
        vec![rows(2, pipeline(all, row))]
    };
    match staged {
        Staged::WalkChildren => root(
            vec![
                walk(tree, TreeWalk::Children(None), 0, 2),
                node_read(tree, NodeColumn::Field, 2, 3),
            ],
            [of(2), of(3)],
        ),
        Staged::WalkField => root(
            vec![
                walk(tree, TreeWalk::Children(Some(FieldCode(4))), 0, 2),
                node_read(tree, NodeColumn::Start, 2, 3),
            ],
            [of(2), of(3)],
        ),
        Staged::WalkDescendants => root(
            vec![
                walk(tree, TreeWalk::Axis(AxisDirection::Descendant), 0, 2),
                node_read(tree, NodeColumn::Kind, 2, 3),
            ],
            [of(2), of(3)],
        ),
        Staged::WalkAncestors => {
            let mut all = Vec::from(scan_nodes(tree));
            all.push(keep_equal(1, 13));
            all.push(walk(tree, TreeWalk::Axis(AxisDirection::Ancestor), 0, 2));
            vec![rows(2, pipeline(all, [of(0), of(2)]))]
        }
        Staged::WalkNext => {
            let mut all = Vec::from(scan_nodes(tree));
            all.push(keep_equal(1, 12));
            all.push(walk(tree, TreeWalk::Axis(AxisDirection::NextSibling), 0, 2));
            vec![rows(2, pipeline(all, [of(0), of(2)]))]
        }
        Staged::ReadNode => {
            let mut all = Vec::from(scan_nodes(tree));
            all.push(keep_equal(1, 13));
            for (column, out) in [
                (NodeColumn::Parent, 2),
                (NodeColumn::Field, 3),
                (NodeColumn::Ordinal, 4),
                (NodeColumn::Start, 5),
                (NodeColumn::End, 6),
                (NodeColumn::Span, 7),
                (NodeColumn::File, 8),
                (NodeColumn::Text, 9),
            ] {
                all.push(node_read(tree, column, 0, out));
            }
            vec![table(
                91,
                9,
                MadeFill::Rows(
                    Box::new([pipeline(
                        all,
                        [
                            of(0),
                            of(2),
                            of(3),
                            of(4),
                            of(5),
                            of(6),
                            of(7),
                            of(8),
                            of(9),
                        ],
                    )]),
                    RepeatedRows::Kept,
                ),
            )]
        }
        Staged::ScanOwn => vec![
            rows(
                2,
                pipeline(Vec::from(scan_nodes(Relation::SyntaxTree)), [of(0), of(1)]),
            ),
            rows(2, pipeline(Vec::from(scan_nodes(tree)), [of(0), of(1)])),
        ],
    }
}

fn subject(staged: Input) -> Result<Value, Failure> {
    let made_tables = tables(staged);
    let last = MadeTableId(u32::try_from(made_tables.len().saturating_sub(1)).unwrap_or(0));
    let mut world = world_with(made_tables, vec![definition()], &ROWS, Telling::Keyed)?;
    let (mut frame, looped) = framed(&mut world);
    let _now = made(&mut frame, looped, last)?;
    let mut said = shown(frame.store, last);
    if matches!(staged, Staged::ScanOwn) {
        let _now = made(&mut frame, looped, MadeTableId(0))?;
        said = format!("own: {} | all: {said}", shown(frame.store, MadeTableId(0)));
    }
    let work = table_of(frame.store, last)
        .map(|slot| slot.work)
        .unwrap_or_default();
    let keys = if looped.keys.is_empty() {
        String::new()
    } else {
        format!(" | asked {:?}", looped.keys)
    };
    Ok(Cow::Owned(format!(
        "{said} | read {} given {} made {}{keys}",
        work.read.0, work.given.0, work.made.0
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
