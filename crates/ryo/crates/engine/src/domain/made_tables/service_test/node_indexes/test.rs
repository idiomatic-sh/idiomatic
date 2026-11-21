//! The cases of a probe of the file's nodes by one of their columns, each run through `made` on a pack the contract's
//! encoding wrote: the index of the nodes by id, by parent, by kind and by field, over the file's own tree and over
//! every element. The file: a root (kind 11) with three children (kind 12, the second under field 24), each holding one
//! token (kind 13), and one embedded element (kind 12) past the own tree. A case says, for each node of the scan, the
//! nodes the probe found for it, and how many rows the arena's indexes hold.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::filled::errors::FilledError;
use crate::domain::lowered::types::MadeTable;
use crate::domain::made_tables::fixtures::packs::{
    node_read, of, operands, pipeline, rows, scan_nodes,
};
use crate::domain::made_tables::fixtures::{Telling, framed, shown, world_with};
use crate::domain::made_tables::sources::service::made;
use crate::domain::steps::stages::types::{FillStage, UseMode};
use crate::domain::steps::types::{MadeTableId, MatchColumns, ReadColumn, Register, StepRows};
use ryo_relations::domain::relations::types::{NodeColumn, Relation};
use std::borrow::Cow;

/// The column a case's probe reads the nodes by, and whether over the own tree alone.
#[derive(Clone, Copy, Debug)]
enum Probed {
    /// By id, asked with each node's parent: a node's parent.
    Id,
    /// By parent, asked with each node's id: a node's children.
    Parent,
    /// By parent over the own tree alone.
    ParentOwn,
    /// By kind, asked with each node's kind: the nodes of its kind.
    Kind,
    /// By kind over the own tree alone.
    KindOwn,
    /// By field, asked with each node's field: the nodes under its field.
    Field,
}

type Input = Probed;
type Value = Cow<'static, str>;
type Failure = FilledError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_nodes_by_id_are_each_found_at_its_own_place"),
        word: Word::Succeeds,
        input: || Probed::Id,
        expect: Expect::Value(Cow::Borrowed(
            "0>0 1>0 2>0 3>0 4>1 5>2 6>3 7>3 | index rows 8",
        )),
    },
    Case {
        name: CaseName("the_nodes_by_parent_are_each_nodes_children_and_the_root_its_own"),
        word: Word::Succeeds,
        input: || Probed::Parent,
        expect: Expect::Value(Cow::Borrowed(
            "0>0 0>1 0>2 0>3 1>4 2>5 3>6 3>7 | index rows 8",
        )),
    },
    Case {
        name: CaseName("the_nodes_by_parent_over_the_own_tree_leave_embedded_elements_out"),
        word: Word::Succeeds,
        input: || Probed::ParentOwn,
        expect: Expect::Value(Cow::Borrowed("0>0 0>1 0>2 0>3 1>4 2>5 3>6 | index rows 7")),
    },
    Case {
        name: CaseName("the_nodes_by_kind_are_the_nodes_of_each_kind_ascending"),
        word: Word::Succeeds,
        input: || Probed::Kind,
        expect: Expect::Value(Cow::Borrowed(
            "0>0 1>1 1>2 1>3 1>7 2>1 2>2 2>3 2>7 3>1 3>2 3>3 3>7 4>4 4>5 4>6 5>4 5>5 5>6 6>4 6>5 6>6 7>1 7>2 7>3 7>7 | index rows 8",
        )),
    },
    Case {
        name: CaseName("the_nodes_by_kind_over_the_own_tree_leave_embedded_elements_out"),
        word: Word::Succeeds,
        input: || Probed::KindOwn,
        expect: Expect::Value(Cow::Borrowed(
            "0>0 1>1 1>2 1>3 2>1 2>2 2>3 3>1 3>2 3>3 4>4 4>5 4>6 5>4 5>5 5>6 6>4 6>5 6>6 | index rows 7",
        )),
    },
    Case {
        name: CaseName("the_nodes_by_field_are_the_nodes_under_each_field"),
        word: Word::Succeeds,
        input: || Probed::Field,
        expect: Expect::Value(Cow::Borrowed(
            "0>0 0>1 0>3 0>4 0>5 0>6 0>7 1>0 1>1 1>3 1>4 1>5 1>6 1>7 2>2 3>0 3>1 3>3 3>4 3>5 3>6 3>7 4>0 4>1 4>3 4>4 4>5 4>6 4>7 5>0 5>1 5>3 5>4 5>5 5>6 5>7 6>0 6>1 6>3 6>4 6>5 6>6 6>7 7>0 7>1 7>3 7>4 7>5 7>6 7>7 | index rows 8",
        )),
    },
];

/// The table of a case: a scan of the nodes, the column the probe is asked with, the probe, and the write of each node
/// with a node found for it.
fn table_of_case(probed: Probed) -> MadeTable {
    let (relation, by, asked) = match probed {
        Probed::Id => (Relation::SemanticSyntax, NodeColumn::Id, NodeColumn::Parent),
        Probed::Parent => (Relation::SemanticSyntax, NodeColumn::Parent, NodeColumn::Id),
        Probed::ParentOwn => (Relation::SyntaxTree, NodeColumn::Parent, NodeColumn::Id),
        Probed::Kind => (Relation::SemanticSyntax, NodeColumn::Kind, NodeColumn::Kind),
        Probed::KindOwn => (Relation::SyntaxTree, NodeColumn::Kind, NodeColumn::Kind),
        Probed::Field => (
            Relation::SemanticSyntax,
            NodeColumn::Field,
            NodeColumn::Field,
        ),
    };
    let mut all = Vec::from(scan_nodes(relation));
    all.push(node_read(relation, asked, 0, 2));
    all.push(FillStage::Probe {
        source: StepRows::Node(relation),
        columns: MatchColumns(Box::new([ReadColumn::Node(by)])),
        values: operands([of(2)]),
        range: None,
        mode: UseMode::Rows,
        out: Register(3),
    });
    rows(2, pipeline(all, [of(0), of(3)]))
}

fn subject(probed: Input) -> Result<Value, Failure> {
    let mut world = world_with(vec![table_of_case(probed)], Vec::new(), &[], Telling::Keyed)?;
    let (mut frame, looped) = framed(&mut world);
    let _now = made(&mut frame, looped, MadeTableId(0))?;
    // Each row as the two nodes' places in the file.
    let said: Vec<String> = shown(frame.store, MadeTableId(0))
        .split("; ")
        .map(|row| {
            row.split(' ')
                .map(|cell| (cell.parse::<u64>().unwrap_or(0) & 0xffff_ffff).to_string())
                .collect::<Vec<_>>()
                .join(">")
        })
        .collect();
    Ok(Cow::Owned(format!(
        "{} | index rows {}",
        said.join(" "),
        frame.store.indexes.rows.len()
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
