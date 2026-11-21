//! The `ordering` pass's cases: a level of a unit the statistics measured takes the order its measured ranks give, under its
//! register dependencies; an anchor, a step that ends a row and every step the sample did not reach keep their places;
//! a unit the statistics do not name, name with other plan points or deny keeps the lowering's order. A row says the
//! unit's steps in the order the shared walk meets them after the pass, each as the register it writes (`-` when it
//! writes none) and its plan point, which it keeps wherever it stands. The runs of a row are by plan point.

use super::service::passed;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{FillCode, MeasuredForm, ShapeChoice, UnitKey};
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill,
    MadeTable, PackDefinition, PatternForm,
};
use crate::domain::passes::numbering;
use crate::domain::statistics::types::{
    LearnStage, MeasuredSizes, MeasuredUnit, OrderVerdict, Statistics, UnitCounts,
};
use crate::domain::steps::traversal::service::each_step;
use crate::domain::steps::types::{
    AnchorKinds, ColumnMask, EmitTarget, MadeTableId, MatchColumns, PlacedStep, Polarity,
    ProgramStep, ReadColumn, Register, RegisterList, StepBody, StepMemo, StepOperand, StepOperands,
    StepProgram, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{Count, KindBits, NodeColumn, Relation};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// The unit a row orders.
#[derive(Clone, Copy, Debug)]
enum Level {
    /// A rule's branch: an anchor (1), the children of its node (2), a read of the node (3), a test of what was read, an
    /// emit.
    Walk,
    /// The same steps after the formals of a pattern's call form, ending in a yield: no step of it is an anchor.
    Pattern,
    /// A rule's branch: an anchor (1), an `Exists` over the children (4) of the node, a read of each (5), a test and a
    /// yield; then a read of the node (3), a test of it and an emit.
    Body,
    /// A rule's branch: an anchor (1), the rows of a made table the node keys (2), a test of the node, an emit.
    Table,
    /// A rule's branch whose first step is no file-sized one: the pattern's steps, ending in an emit.
    Keyed,
    /// A rule's branch: an anchor (1), the children of its node (2), an emit, a read of the node (3), a test, an emit.
    Emits,
    /// A rule's branch: an anchor (1), an `Exists` whose body writes registers 4 and 5, then a read of the node into
    /// register 5 and a test of it, and an emit.
    Scratch,
}

/// What the statistics say of the unit.
#[derive(Clone, Copy, Debug)]
enum Said {
    /// No statistics.
    Nothing,
    /// These runs, by plan point, under the unit's own name.
    Runs(&'static [usize]),
    /// These runs with the order denied.
    Denied(&'static [usize]),
    /// These runs under the name of another node.
    Other(&'static [usize]),
}

type Input = (Level, Said);
type Value = Cow<'static, str>;
type Failure = ();

/// A node of 100 children: the anchor runs once, the walk 100 times, and the read and the test once for each child; one
/// child in ten passes the test.
const WALK: &[usize] = &[1, 100, 1000, 1000, 100];

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("with_no_statistics_every_level_keeps_the_lowering_order"),
        word: Word::Succeeds,
        input: || (Level::Walk, Said::Nothing),
        expect: Expect::Value(Cow::Borrowed("1@0 2@1 3@2 -@3 -@4")),
    },
    Case {
        name: CaseName(
            "a_selective_test_goes_before_a_walk_and_never_before_the_read_that_feeds_it",
        ),
        word: Word::Succeeds,
        input: || (Level::Walk, Said::Runs(WALK)),
        expect: Expect::Value(Cow::Borrowed("1@0 3@2 -@3 2@1 -@4")),
    },
    Case {
        name: CaseName("a_pattern_s_call_form_is_ordered_from_its_first_step"),
        word: Word::Succeeds,
        input: || (Level::Pattern, Said::Runs(&[100, 1000, 1000, 100])),
        expect: Expect::Value(Cow::Borrowed("3@1 -@2 2@0 -@3")),
    },
    Case {
        name: CaseName(
            "a_step_the_sample_did_not_reach_keeps_its_place_and_so_does_every_step_after_it",
        ),
        word: Word::Succeeds,
        input: || (Level::Walk, Said::Runs(&[1, 100, 0, 0, 0])),
        expect: Expect::Value(Cow::Borrowed("1@0 2@1 3@2 -@3 -@4")),
    },
    Case {
        name: CaseName("a_denied_order_is_not_taken"),
        word: Word::Succeeds,
        input: || (Level::Walk, Said::Denied(WALK)),
        expect: Expect::Value(Cow::Borrowed("1@0 2@1 3@2 -@3 -@4")),
    },
    Case {
        name: CaseName("runs_of_another_lowering_of_the_node_are_ignored"),
        word: Word::Succeeds,
        input: || (Level::Walk, Said::Runs(&[1, 100, 1000, 1000, 100, 100])),
        expect: Expect::Value(Cow::Borrowed("1@0 2@1 3@2 -@3 -@4")),
    },
    Case {
        name: CaseName("runs_of_another_node_are_not_applied"),
        word: Word::Succeeds,
        input: || (Level::Walk, Said::Other(WALK)),
        expect: Expect::Value(Cow::Borrowed("1@0 2@1 3@2 -@3 -@4")),
    },
    Case {
        name: CaseName("an_inline_body_s_steps_are_ordered_too_and_move_with_their_step"),
        word: Word::Succeeds,
        input: || {
            (
                Level::Body,
                Said::Runs(&[1, 100, 100, 1000, 1000, 100, 100, 100, 10]),
            )
        },
        expect: Expect::Value(Cow::Borrowed("1@0 3@6 -@7 -@1 4@2 5@3 -@4 -@5 -@8")),
    },
    Case {
        name: CaseName("a_table_s_fill_is_its_reader_s_work_so_a_cheap_test_goes_first"),
        word: Word::Succeeds,
        input: || (Level::Table, Said::Runs(&[1, 100, 50, 30])),
        expect: Expect::Value(Cow::Borrowed("1@0 -@2 2@1 -@3")),
    },
    Case {
        name: CaseName("the_first_step_of_a_rule_s_branch_keeps_its_place_whatever_its_class"),
        word: Word::Succeeds,
        input: || (Level::Keyed, Said::Runs(&[100, 1000, 1000, 100])),
        expect: Expect::Value(Cow::Borrowed("2@0 3@1 -@2 -@3")),
    },
    Case {
        name: CaseName("no_step_moves_past_a_step_that_ends_a_row"),
        word: Word::Succeeds,
        input: || (Level::Emits, Said::Runs(&[1, 100, 1000, 1000, 1000, 100])),
        expect: Expect::Value(Cow::Borrowed("1@0 2@1 -@2 3@3 -@4 -@5")),
    },
    Case {
        name: CaseName("a_step_never_goes_before_a_step_whose_body_writes_a_register_it_writes"),
        word: Word::Succeeds,
        input: || {
            (
                Level::Scratch,
                Said::Runs(&[1, 100, 100, 1000, 1000, 100, 100, 100, 10]),
            )
        },
        expect: Expect::Value(Cow::Borrowed("1@0 -@1 4@2 5@3 -@4 -@5 5@6 -@7 -@8")),
    },
];

/// The node the rows' unit is named by, and the node of the made table.
const NODE: NodeDigest = NodeDigest([7; 16]);
const TABLE_NODE: NodeDigest = NodeDigest([9; 16]);
const TREE: Relation = Relation::SyntaxTree;

fn read(row: u32, out: u32) -> ProgramStep {
    ProgramStep::Read {
        source: StepRows::Node(TREE),
        column: ReadColumn::Node(NodeColumn::Kind),
        row: Register(row),
        out: Register(out),
    }
}

fn test(left: u32) -> ProgramStep {
    ProgramStep::Test {
        test: TestOperation::Equal,
        left: StepOperand::Register(Register(left)),
        right: StepOperand::Word(CellWord(1)),
    }
}

fn children(parent: u32, out: u32) -> ProgramStep {
    ProgramStep::Children {
        relation: TREE,
        parent: Register(parent),
        field: None,
        out: Register(out),
    }
}

fn anchor() -> ProgramStep {
    ProgramStep::EachKind {
        relation: TREE,
        kinds: AnchorKinds(Box::new([KindBits(1)])),
        out: Register(1),
    }
}

fn emit() -> ProgramStep {
    ProgramStep::Emit {
        target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
        columns: StepOperands::default(),
    }
}

fn program(steps: Vec<ProgramStep>) -> StepProgram {
    let placed = steps
        .into_iter()
        .map(|step| PlacedStep { point: None, step });
    StepProgram(placed.collect())
}

/// The steps of a row's level.
fn steps(level: Level) -> Vec<ProgramStep> {
    let yielded = || ProgramStep::Yield {
        row: RegisterList::default(),
    };
    match level {
        Level::Walk => vec![anchor(), children(1, 2), read(1, 3), test(3), emit()],
        Level::Pattern => vec![children(1, 2), read(1, 3), test(3), yielded()],
        Level::Keyed => vec![children(1, 2), read(1, 3), test(3), emit()],
        Level::Emits => vec![
            anchor(),
            children(1, 2),
            emit(),
            read(1, 3),
            test(3),
            emit(),
        ],
        Level::Body | Level::Scratch => {
            let body = program(vec![children(1, 4), read(4, 5), test(5), yielded()]);
            let exists = ProgramStep::Exists {
                polarity: Polarity::Present,
                body: StepBody::Inline(body),
                memo: StepMemo::None,
            };
            let after = match level {
                Level::Scratch => 5,
                Level::Walk
                | Level::Pattern
                | Level::Body
                | Level::Table
                | Level::Keyed
                | Level::Emits => 3,
            };
            vec![anchor(), exists, read(1, after), test(after), emit()]
        }
        Level::Table => {
            let rows = ProgramStep::EachMatch {
                source: StepRows::Made(MadeTableId(0)),
                columns: MatchColumns::default(),
                values: StepOperands(Box::new([StepOperand::Register(Register(1))])),
                out: Register(2),
            };
            vec![anchor(), rows, test(1), emit()]
        }
    }
}

/// The pack of a row: its level as a rule's one branch, or as the call form of a definition's one pattern; and one made
/// table, of written rows.
fn pack(level: Level) -> LoweredPack {
    let mut pack = LoweredPack {
        tables: vec![MadeTable {
            node: TABLE_NODE,
            fill: MadeFill::Constant(ConstantRows::default()),
            width: ColumnCount(1),
            indexes: Vec::new(),
            emptiness: Emptiness::Unknown,
        }],
        ..LoweredPack::default()
    };
    match level {
        Level::Pattern => pack.definitions.push(PackDefinition {
            node: NODE,
            formals: RegisterList::default(),
            emptiness: Emptiness::Unknown,
            patterns: vec![DefinitionPattern {
                bound: ColumnMask::default(),
                collected: ColumnMask::default(),
                form: PatternForm::Call(program(steps(level))),
                shape: ShapeChoice::Decided,
            }],
        }),
        Level::Walk | Level::Body | Level::Table | Level::Keyed | Level::Emits | Level::Scratch => {
            pack.rules.push(LoweredRule {
                rule: SelectedRuleOrdinal(0),
                node: NODE,
                branches: vec![Branch {
                    emptiness: Emptiness::Unknown,
                    program: program(steps(level)),
                }],
                columns: RuleColumns::default(),
            })
        }
    }
    pack
}

/// A unit as a sample measured it.
fn unit(key: UnitKey, order: OrderVerdict, runs: &[usize], counts: UnitCounts) -> MeasuredUnit {
    MeasuredUnit {
        key,
        form: MeasuredForm::Call,
        shape: ShapeChoice::Decided,
        order,
        points: runs.iter().map(|each| Count(*each)).collect(),
        counts,
    }
}

/// The statistics a row is ordered with: the row's unit under a name, and the made table, whose fill read and gave 4,000
/// rows over its 4 reads.
fn statistics(level: Level, said: Said) -> Option<Statistics> {
    let named = |node| match level {
        Level::Pattern => UnitKey::Pattern(node, ColumnMask::default(), ColumnMask::default()),
        Level::Walk | Level::Body | Level::Table | Level::Keyed | Level::Emits | Level::Scratch => {
            UnitKey::Rule(node)
        }
    };
    let none = UnitCounts::default();
    let measured = match said {
        Said::Nothing => return None,
        Said::Runs(runs) => unit(named(NODE), OrderVerdict::Learned, runs, none),
        Said::Denied(runs) => unit(named(NODE), OrderVerdict::Denied, runs, none),
        Said::Other(runs) => unit(
            named(NodeDigest([8; 16])),
            OrderVerdict::Learned,
            runs,
            none,
        ),
    };
    let fill = UnitCounts {
        calls: Count(4),
        build: Count(4000),
        ..none
    };
    let table = UnitKey::Table(TABLE_NODE, FillCode(11));
    let mut units = vec![measured, unit(table, OrderVerdict::Learned, &[], fill)];
    units.sort_by(|left, right| left.key.cmp(&right.key));
    Some(Statistics {
        stage: LearnStage::First,
        sizes: MeasuredSizes::default(),
        units,
    })
}

fn subject((level, said): Input) -> Result<Value, Failure> {
    let mut pack = pack(level);
    numbering::service::passed(&mut pack);
    passed(&mut pack, statistics(level, said).as_ref());
    let patterns = pack
        .definitions
        .iter()
        .flat_map(|definition| &definition.patterns);
    let called = patterns.filter_map(|pattern| match &pattern.form {
        PatternForm::Call(program) => Some(program),
        PatternForm::Table(_) => None,
    });
    let branches = pack.rules.iter().flat_map(|rule| &rule.branches);
    let mut said = Vec::new();
    for program in called.chain(branches.map(|branch| &branch.program)) {
        each_step(program, &mut |placed, touched| {
            let wrote = touched.writes.first();
            let wrote = wrote.map_or_else(|| "-".to_owned(), |register| register.0.to_string());
            let point = placed
                .point
                .map_or_else(|| "none".to_owned(), |point| point.0.to_string());
            said.push(format!("{wrote}@{point}"));
        });
    }
    Ok(Cow::Owned(said.join(" ")))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
