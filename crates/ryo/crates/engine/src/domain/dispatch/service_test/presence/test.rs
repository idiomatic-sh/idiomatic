//! The cases of `dispatch` over a need on a node relation's kind or field. A file's tree is a root of kind 1 with one
//! child of kind 2, under field 4 or under none, and, when the case says so, one embedded segment whose element is of
//! kind 3 under field 4: past the file's own tree. The vocabulary's kinds 0 to 3 have the words 200 to 203 and its fields
//! 0 to 4 the words 100 to 104. Two rules read one table, whose pipeline scans the node rows of a relation and whose need
//! says those rows hold a row with a given kind or field (or any row). A table whose need does not hold is never made,
//! and no stage of its fill runs.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, Emptiness, LoweredPack, LoweredRule, MadeTable, RowValue,
};
use crate::domain::made_tables::fixtures::packs::{of, operands, pipeline, rows, scan_nodes};
use crate::domain::made_tables::sources::service::table_of;
use crate::domain::programs::fixtures::{Held, framed_over};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::scratch::service::next_file;
use crate::domain::steps::types::{
    EmitTarget, MadeTableId, PlacedStep, ProgramStep, Register, RegisterCount, StepProgram,
    StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::service::{builder, embed, enter_node, finished, leave_node};
use ryo_language::domain::node_table::types::{EmbeddedText, FieldCode, NodeTable};
use ryo_relations::domain::elements::types::EmbeddedMapping;
use ryo_relations::domain::relations::collections::relations::semantic_child::item::SemanticChildColumn;
use ryo_relations::domain::relations::collections::relations::semantic_syntax::item::SemanticSyntaxColumn;
use ryo_relations::domain::relations::collections::relations::syntax_child::item::SyntaxChildColumn;
use ryo_relations::domain::relations::collections::relations::syntax_tree::item::SyntaxTreeColumn;
use ryo_relations::domain::relations::types::{
    Column, Count, KindBits, Offset, Position, Relation,
};
use ryo_relations::domain::terms::types::NameText;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// What the table's need asks of the node rows.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A row of every tree's rows (`semantic-child`) under the field of a word.
    FieldOfEveryRow(u64),
    /// A row of the own tree's rows (`syntax-child`) under the field of a word.
    FieldOfTheOwnTree(u64),
    /// A row of every tree's rows (`semantic-syntax`) of the kind of a word.
    KindOfEveryRow(u64),
    /// A row of the own tree's rows (`syntax-tree`) of the kind of a word.
    KindOfTheOwnTree(u64),
    /// Any row of every tree's rows.
    AnyRow,
}

/// One file's tree: the field its one child is under, and whether it has the embedded segment.
#[derive(Clone, Copy, Debug)]
enum Tree {
    /// The child is under field 4; no embedded segment.
    FieldedChild,
    /// The child is under no field; no embedded segment.
    PlainChild,
    /// The child is under no field; the embedded element is of kind 3 under field 4.
    PlainChildAndAnEmbeddedOne,
}

/// What the need asks, and the files one worker scans, in order.
type Input = (Asked, &'static [Tree]);
/// For each file the rows the two rules found, whether the table was made (1), and the rows the fills' stages read and
/// gave; then, of the last file, the terms of the second rule's allowance beyond its candidates (the node table's
/// elements once it has read it; the rows into and out of the table).
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_field_some_element_is_under_makes_the_table"),
        word: Word::Succeeds,
        input: || (Asked::FieldOfEveryRow(104), &[Tree::FieldedChild]),
        expect: Expect::Value(Cow::Borrowed(&[4, 1, 10, 6])),
    },
    Case {
        name: CaseName("a_field_no_element_is_under_makes_no_table_and_runs_no_stage"),
        word: Word::Succeeds,
        input: || (Asked::FieldOfEveryRow(103), &[Tree::FieldedChild]),
        expect: Expect::Value(Cow::Borrowed(&[0, 0, 0, 2])),
    },
    Case {
        name: CaseName("a_word_that_is_no_field_makes_no_table"),
        word: Word::Succeeds,
        input: || (Asked::FieldOfEveryRow(555), &[Tree::FieldedChild]),
        expect: Expect::Value(Cow::Borrowed(&[0, 0, 0, 2])),
    },
    Case {
        name: CaseName("a_need_for_any_row_holds_in_a_file_with_elements"),
        word: Word::Succeeds,
        input: || (Asked::AnyRow, &[Tree::PlainChild]),
        expect: Expect::Value(Cow::Borrowed(&[4, 1, 10, 6])),
    },
    Case {
        name: CaseName("a_field_only_an_embedded_element_is_under_is_not_in_the_own_tree"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::FieldOfTheOwnTree(104),
                &[Tree::PlainChildAndAnEmbeddedOne],
            )
        },
        expect: Expect::Value(Cow::Borrowed(&[0, 0, 0, 3])),
    },
    Case {
        name: CaseName("a_field_only_an_embedded_element_is_under_is_in_every_row"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::FieldOfEveryRow(104),
                &[Tree::PlainChildAndAnEmbeddedOne],
            )
        },
        expect: Expect::Value(Cow::Borrowed(&[6, 1, 15, 9])),
    },
    Case {
        name: CaseName("a_kind_only_an_embedded_element_has_is_not_in_the_own_tree"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::KindOfTheOwnTree(203),
                &[Tree::PlainChildAndAnEmbeddedOne],
            )
        },
        expect: Expect::Value(Cow::Borrowed(&[0, 0, 0, 3])),
    },
    Case {
        name: CaseName("a_kind_only_an_embedded_element_has_is_in_every_row"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::KindOfEveryRow(203),
                &[Tree::PlainChildAndAnEmbeddedOne],
            )
        },
        expect: Expect::Value(Cow::Borrowed(&[6, 1, 15, 9])),
    },
    Case {
        name: CaseName("a_kind_of_the_own_tree_makes_the_table"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::KindOfTheOwnTree(202),
                &[Tree::PlainChildAndAnEmbeddedOne],
            )
        },
        expect: Expect::Value(Cow::Borrowed(&[4, 1, 10, 8])),
    },
    Case {
        name: CaseName("the_next_file_is_asked_anew_what_the_file_before_held_is_gone"),
        word: Word::Succeeds,
        input: || {
            (
                Asked::FieldOfEveryRow(104),
                &[Tree::FieldedChild, Tree::PlainChild, Tree::FieldedChild],
            )
        },
        expect: Expect::Value(Cow::Borrowed(&[4, 1, 10, 0, 0, 0, 4, 1, 10, 6])),
    },
];

/// A file's tree.
fn tree(shape: Tree) -> NodeTable {
    let mut made = NodeTable::default();
    let mut writer = builder(&mut made, Count(4));
    let field = match shape {
        Tree::FieldedChild => FieldCode(4),
        Tree::PlainChild | Tree::PlainChildAndAnEmbeddedOne => FieldCode(0),
    };
    enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
    enter_node(&mut writer, KindBits(2), field, Offset(0));
    leave_node(&mut writer, Offset(2));
    leave_node(&mut writer, Offset(8));
    if matches!(shape, Tree::PlainChildAndAnEmbeddedOne) {
        let text = EmbeddedText {
            text: NameText("x"),
            mapping: EmbeddedMapping::Exact(Offset(0)),
        };
        let _root = embed(&mut writer, Position(1), text);
        enter_node(&mut writer, KindBits(3), FieldCode(4), Offset(0));
        leave_node(&mut writer, Offset(1));
    }
    let _table = finished(&mut writer);
    made
}

/// The rule set: the one table, read by two rules.
fn pack(asked: Asked) -> LoweredPack {
    let read = |number: u32| {
        let steps = [
            ProgramStep::EachRow {
                source: StepRows::Made(MadeTableId(0)),
                out: Register(900_u32.saturating_add(number)),
            },
            ProgramStep::Emit {
                target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
                columns: operands([]),
            },
        ];
        LoweredRule {
            rule: SelectedRuleOrdinal(number),
            node: NodeDigest([u8::try_from(number).unwrap_or(u8::MAX); 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Unknown,
                program: StepProgram(
                    steps
                        .into_iter()
                        .map(|step| PlacedStep { point: None, step })
                        .collect(),
                ),
            }],
            columns: RuleColumns::default(),
        }
    };
    let (relation, value) = match asked {
        Asked::FieldOfEveryRow(word) => (
            Relation::SemanticChild,
            Some((Column::SemanticChild(SemanticChildColumn::Field), word)),
        ),
        Asked::FieldOfTheOwnTree(word) => (
            Relation::SyntaxChild,
            Some((Column::SyntaxChild(SyntaxChildColumn::Field), word)),
        ),
        Asked::KindOfEveryRow(word) => (
            Relation::SemanticSyntax,
            Some((Column::SemanticSyntax(SemanticSyntaxColumn::Kind), word)),
        ),
        Asked::KindOfTheOwnTree(word) => (
            Relation::SyntaxTree,
            Some((Column::SyntaxTree(SyntaxTreeColumn::Kind), word)),
        ),
        Asked::AnyRow => (Relation::SemanticChild, None),
    };
    let value = value.map(|(column, word)| RowValue {
        column,
        value: CellWord(word),
    });
    let stages = Vec::from(scan_nodes(relation));
    LoweredPack {
        tables: vec![MadeTable {
            emptiness: Emptiness::Row(relation, value),
            ..rows(1, pipeline(stages, [of(1)]))
        }],
        rules: vec![read(0), read(1)],
        registers: RegisterCount(1000),
        ..LoweredPack::default()
    }
}

fn subject((asked, files): Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(asked)).map_err(|_| ())?,
        kinds: (200..204).map(CellWord).collect(),
        fields: (100..105).map(CellWord).collect(),
        ..ProgramsEngine::default()
    });
    let mut worker = scratch_for(&programs);
    let (mut said, mut terms) = (Vec::new(), 0);
    for shape in files {
        let nodes = tree(*shape);
        next_file(&mut worker);
        // The loop's fixtures frame a state and a store: the worker's are lent to them for the file.
        let mut held = Held {
            state: std::mem::take(&mut worker.0.state),
            store: std::mem::take(&mut worker.0.store),
        };
        let (counted, found) = framed_over(&nodes, &programs.0, &mut held, &TheLoop, |frame| {
            dispatch(frame, &programs)
        });
        said.extend([
            u64::try_from(found.rows.len()).unwrap_or(u64::MAX),
            u64::from(table_of(&held.store, MadeTableId(0)).is_some()),
            counted.fill_rows.0,
        ]);
        terms = held.state.budget.terms.0;
        worker.0.state = held.state;
        worker.0.store = held.store;
    }
    said.push(terms);
    Ok(Cow::Owned(said))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
