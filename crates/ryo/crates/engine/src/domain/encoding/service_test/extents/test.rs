//! The flag `OWN_TREE` follows the catalog's extent of the relation a word reads its node rows or walks through: every
//! node-source word and every walk of the tree bears it over a relation of the file's own tree, none bears it over a relation
//! that holds the embedded segments too, and no other word ever does. The packs are the law's, which holds every step and
//! stage, and one of the three reads of node rows the law makes of filled and made rows only (a match, a range, every
//! row), each written once through each relation.

use super::super::fixtures::{operands, pack, reg, rule, word};
use super::super::law::fixtures::every_variant_over;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::constants::OWN_TREE;
use crate::domain::encoding::service::encoded;
use crate::domain::lowered::types::LoweredPack;
use crate::domain::lowering::errors::LoweringError;
use crate::domain::steps::types::{
    BoundKind, MatchColumns, ProgramStep, RangeBound, ReadColumn, StepRows,
};
use ryo_relations::domain::relations::types::{NodeColumn, Relation};
use std::borrow::Cow;
use std::collections::BTreeSet;

/// The relation the law's pack reads its node rows and walks through.
type Input = Relation;
/// The codes whose words bear the flag, each once, in name order.
type Value = Cow<'static, str>;
type Failure = LoweringError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("over_the_files_own_tree_every_node_source_and_walk_bears_the_flag"),
        word: Word::Succeeds,
        input: || Relation::SyntaxTree,
        expect: Expect::Value(Cow::Borrowed(
            "[Axis, Children, ChildrenOfField, EachKind, EachMatchNode, EachRange, EachRowNode, Nearest, StageScan, StageWalk, Within]",
        )),
    },
    Case {
        name: CaseName("over_a_tree_with_the_embedded_segments_no_word_bears_the_flag"),
        word: Word::Succeeds,
        input: || Relation::SemanticSyntax,
        expect: Expect::Value(Cow::Borrowed("[]")),
    },
];

/// A pack of three rules, each one read of the node rows of `node`: by a kind, by a kind within a range of ids, and whole.
fn node_reads(node: Relation) -> LoweredPack {
    let kind = || MatchColumns(Box::new([ReadColumn::Node(NodeColumn::Kind)]));
    let matched = ProgramStep::EachMatch {
        source: StepRows::Node(node),
        columns: kind(),
        values: operands([word(2)]),
        out: reg(0),
    };
    let ranged = ProgramStep::EachRange {
        source: StepRows::Node(node),
        columns: kind(),
        values: operands([word(2)]),
        range: ReadColumn::Node(NodeColumn::Id),
        low: Some(RangeBound {
            value: word(1),
            end: BoundKind::Inclusive,
        }),
        high: None,
        out: reg(1),
        range_out: None,
    };
    let every = ProgramStep::EachRow {
        source: StepRows::Node(node),
        out: reg(2),
    };
    pack(vec![
        rule(0, vec![matched]),
        rule(1, vec![ranged]),
        rule(2, vec![every]),
    ])
}

fn subject(node: Input) -> Result<Value, Failure> {
    let (law, reads) = (
        encoded(&every_variant_over(node))?,
        encoded(&node_reads(node))?,
    );
    let flagged: BTreeSet<String> = law
        .words
        .iter()
        .chain(&reads.words)
        .filter(|word| word.flag.0 & OWN_TREE.0 != 0)
        .map(|word| format!("{:?}", word.op))
        .collect();
    let every: Vec<String> = flagged.into_iter().collect();
    Ok(Cow::Owned(format!("[{}]", every.join(", "))))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
