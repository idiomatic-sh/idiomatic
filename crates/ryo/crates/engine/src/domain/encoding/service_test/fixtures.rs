//! What the encoding's cases share: the smallest parts of a lowered pack, and a code's words read back as text.

use crate::domain::code::anchoring::service::{
    each_kind_of, each_match_filled_many_of, each_match_filled_of, each_match_made_of,
    each_match_node_of, each_range_of, each_row_filled_of, each_row_made_of, each_row_node_of,
    latest_of,
};
use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::controlling::service::{emit_of, exists_of, fork_of, or_of, yield_row_of};
use crate::domain::code::deciding::service::{
    call_of, let_value_of, read_filled_of, read_made_of, read_node_of, render_of,
    test_equal_constant_of, test_equal_register_of, test_in_constants_of,
    test_not_equal_constant_of, test_not_equal_register_of, test_of,
};
use crate::domain::code::filling::service::{
    stage_call_of, stage_keep_of, stage_latest_of, stage_probe_of, stage_product_of, stage_read_of,
    stage_render_of, stage_scan_of, stage_use_of, stage_walk_of, stage_write_of,
};
use crate::domain::code::types::{CodeWord, Op, ProgramCode};
use crate::domain::code::walking::service::{
    axis_of, children_of, children_of_field_of, closure_of, nearest_derived_of, nearest_of,
    within_of,
};
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable,
    PackDefinition,
};
use crate::domain::steps::stages::types::{
    FillStage, Pipeline, PlacedStage, ProbeRange, TreeWalk, UseMode,
};
use crate::domain::steps::types::{
    AnchorKinds, AxisDirection, BoundKind, CallMeaning, ColumnMask, DefinitionId, MadeColumn,
    MadeTableId, MatchColumns, PlacedStep, PlanPoint, ProgramStep, RangeBound, ReadColumn,
    Register, RegisterCount, RegisterList, StepOperand, StepOperands, StepProgram, StepRows,
    TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::FieldCode;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::collections::relations::syntax_error::item::SyntaxErrorColumn;
use ryo_relations::domain::relations::types::{Column, KindBits, Relation};
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// A word's operands as its own reader reads them: one arm for each code.
fn read(word: CodeWord) -> String {
    match word.op {
        Op::End => "no operand".to_owned(),
        Op::EachKind => format!("{:?}", each_kind_of(word)),
        Op::EachMatchNode => format!("{:?}", each_match_node_of(word)),
        Op::EachMatchFilled => format!("{:?}", each_match_filled_of(word)),
        Op::EachMatchFilledMany => format!("{:?}", each_match_filled_many_of(word)),
        Op::EachMatchMade => format!("{:?}", each_match_made_of(word)),
        Op::EachRange => format!("{:?}", each_range_of(word)),
        Op::EachRowNode => format!("{:?}", each_row_node_of(word)),
        Op::EachRowFilled => format!("{:?}", each_row_filled_of(word)),
        Op::EachRowMade => format!("{:?}", each_row_made_of(word)),
        Op::Children => format!("{:?}", children_of(word)),
        Op::ChildrenOfField => format!("{:?}", children_of_field_of(word)),
        Op::Axis => format!("{:?}", axis_of(word)),
        Op::Within => format!("{:?}", within_of(word)),
        Op::Nearest => format!("{:?}", nearest_of(word)),
        Op::NearestDerived => format!("{:?}", nearest_derived_of(word)),
        Op::Closure => format!("{:?}", closure_of(word)),
        Op::ReadNode => format!("{:?}", read_node_of(word)),
        Op::ReadFilled => format!("{:?}", read_filled_of(word)),
        Op::ReadMade => format!("{:?}", read_made_of(word)),
        Op::Test => format!("{:?}", test_of(word)),
        Op::TestEqualConstant => format!("{:?}", test_equal_constant_of(word)),
        Op::TestNotEqualConstant => format!("{:?}", test_not_equal_constant_of(word)),
        Op::TestEqualRegister => format!("{:?}", test_equal_register_of(word)),
        Op::TestNotEqualRegister => format!("{:?}", test_not_equal_register_of(word)),
        Op::TestInConstants => format!("{:?}", test_in_constants_of(word)),
        Op::Call => format!("{:?}", call_of(word)),
        Op::Render => format!("{:?}", render_of(word)),
        Op::Let => format!("{:?}", let_value_of(word)),
        Op::Exists => format!("{:?}", exists_of(word)),
        Op::Or => format!("{:?}", or_of(word)),
        Op::Fork => format!("{:?}", fork_of(word)),
        Op::Yield => format!("{:?}", yield_row_of(word)),
        Op::Latest => format!("{:?}", latest_of(word)),
        Op::Emit => format!("{:?}", emit_of(word)),
        Op::StageScan => format!("{:?}", stage_scan_of(word)),
        Op::StageProbe => format!("{:?}", stage_probe_of(word)),
        Op::StageLatest => format!("{:?}", stage_latest_of(word)),
        Op::StageRead => format!("{:?}", stage_read_of(word)),
        Op::StageKeep => format!("{:?}", stage_keep_of(word)),
        Op::StageCall => format!("{:?}", stage_call_of(word)),
        Op::StageProduct => format!("{:?}", stage_product_of(word)),
        Op::StageUse => format!("{:?}", stage_use_of(word)),
        Op::StageRender => format!("{:?}", stage_render_of(word)),
        Op::StageWalk => format!("{:?}", stage_walk_of(word)),
        Op::StageWrite => format!("{:?}", stage_write_of(word)),
    }
}

/// Every word of a code, one line each: its place, its operation, its operands as its reader reads them, its plan point.
pub(super) fn listing(code: &ProgramCode) -> String {
    let mut lines = String::new();
    for (place, (word, point)) in code.words.iter().zip(&code.points).enumerate() {
        let counted = if *point == NO_SLOT {
            "-".to_owned()
        } else {
            point.0.to_string()
        };
        lines.push_str(&format!(
            "{place} {:?} {} @{counted}\n",
            word.op,
            read(*word)
        ));
    }
    lines
}

/// A register.
pub(super) const fn reg(number: u32) -> Register {
    Register(number)
}

/// A register's value as an operand.
pub(super) const fn of(number: u32) -> StepOperand {
    StepOperand::Register(Register(number))
}

/// A constant as an operand.
pub(super) const fn word(bits: u64) -> StepOperand {
    StepOperand::Word(CellWord(bits))
}

/// StepOperands.
pub(super) fn operands<const N: usize>(each: [StepOperand; N]) -> StepOperands {
    StepOperands(Box::new(each))
}

/// Registers.
pub(super) fn regs<const N: usize>(each: [u32; N]) -> RegisterList {
    RegisterList(each.into_iter().map(Register).collect())
}

/// Columns of a definition.
pub(super) fn mask<const N: usize>(each: [u16; N]) -> ColumnMask {
    ColumnMask(each.into_iter().map(MadeColumn).collect())
}

/// A program of steps, each counted at its place.
pub(super) fn program(steps: Vec<ProgramStep>) -> StepProgram {
    let placed = |(place, step)| PlacedStep {
        point: u32::try_from(place).ok().map(PlanPoint),
        step,
    };
    StepProgram(steps.into_iter().enumerate().map(placed).collect())
}

/// Stages, each counted at its place.
pub(super) fn stages(each: Vec<FillStage>) -> Box<[PlacedStage]> {
    let placed = |(place, stage)| PlacedStage {
        point: u32::try_from(place).ok().map(PlanPoint),
        stage,
    };
    each.into_iter().enumerate().map(placed).collect()
}

/// A rule of one branch that runs `steps`.
pub(super) fn rule(ordinal: u32, steps: Vec<ProgramStep>) -> LoweredRule {
    LoweredRule {
        rule: SelectedRuleOrdinal(ordinal),
        node: NodeDigest([u8::try_from(ordinal).unwrap_or(u8::MAX); 16]),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: program(steps),
        }],
        columns: RuleColumns::default(),
    }
}

/// A pack of `rules` and nothing else, naming a hundred registers.
pub(super) fn pack(rules: Vec<LoweredRule>) -> LoweredPack {
    LoweredPack {
        rules,
        registers: RegisterCount(100),
        ..LoweredPack::default()
    }
}

/// An anchor: each id of one kind.
pub(super) fn anchor() -> ProgramStep {
    ProgramStep::EachKind {
        relation: Relation::SyntaxTree,
        kinds: AnchorKinds(Box::new([KindBits(2)])),
        out: reg(0),
    }
}

/// A scan of the node table into a batch column.
pub(super) fn scan(out: u32) -> FillStage {
    FillStage::Scan {
        source: StepRows::Node(Relation::SyntaxTree),
        columns: MatchColumns::default(),
        values: operands([]),
        out: reg(out),
    }
}

/// A pack of one rule that walks table 0, whose fill is one pipeline of `filled` and whose rows have `width` columns; with
/// `patterns` of a definition of two columns when a stage uses one.
pub(super) fn table_pack(
    filled: Vec<FillStage>,
    width: usize,
    patterns: Vec<DefinitionPattern>,
) -> LoweredPack {
    let pipeline = Pipeline {
        stages: stages(filled),
        row: operands([of(1)]),
    };
    let mut made = pack(vec![rule(
        0,
        vec![ProgramStep::EachRow {
            source: StepRows::Made(MadeTableId(0)),
            out: reg(0),
        }],
    )]);
    made.tables = vec![MadeTable {
        node: NodeDigest([9; 16]),
        fill: MadeFill::Rows(Box::new([pipeline]), RepeatedRows::Kept),
        width: ColumnCount(width),
        indexes: Vec::new(),
        emptiness: Emptiness::Unknown,
    }];
    made.definitions = vec![PackDefinition {
        node: NodeDigest([8; 16]),
        formals: regs([40, 41]),
        emptiness: Emptiness::Unknown,
        patterns,
    }];
    made
}

/// A pipeline that holds every stage, the scan first: it reads a filled relation, made tables 1 and 2 and definitions 0 and 1,
/// and its node rows and its walks are through `node`.
pub(super) fn every_stage_over(node: Relation) -> Pipeline {
    let end = |register| RangeBound {
        value: of(register),
        end: BoundKind::Strict,
    };
    Pipeline {
        stages: stages(vec![
            FillStage::Scan {
                source: StepRows::Node(node),
                columns: MatchColumns::default(),
                values: operands([]),
                out: reg(30),
            },
            FillStage::Probe {
                source: StepRows::Filled(Relation::SyntaxError),
                columns: MatchColumns(Box::new([ReadColumn::Filled(Column::SyntaxError(
                    SyntaxErrorColumn::File,
                ))])),
                values: operands([of(30)]),
                range: None,
                mode: UseMode::Rows,
                out: reg(31),
            },
            FillStage::Probe {
                source: StepRows::Made(MadeTableId(1)),
                columns: MatchColumns(Box::new([ReadColumn::Made(MadeColumn(0))])),
                values: operands([word(5)]),
                range: Some(ProbeRange {
                    column: ReadColumn::Made(MadeColumn(1)),
                    low: end(30),
                    high: end(31),
                }),
                mode: UseMode::Present,
                out: reg(32),
            },
            FillStage::Product {
                source: StepRows::Definition(DefinitionId(1)),
                out: reg(33),
            },
            FillStage::Use {
                definition: DefinitionId(0),
                bound: mask([0]),
                collected: mask([1]),
                values: operands([of(30)]),
                mode: UseMode::Rows,
                outputs: regs([34]),
            },
            FillStage::Use {
                definition: DefinitionId(1),
                bound: mask([0]),
                collected: mask([1]),
                values: operands([of(30)]),
                mode: UseMode::Rows,
                outputs: regs([35]),
            },
            FillStage::Latest {
                table: MadeTableId(2),
                key: operands([of(30)]),
                bound: word(5),
                side: OrderBound::After,
                out: reg(36),
            },
            FillStage::Read {
                source: StepRows::Made(MadeTableId(2)),
                column: ReadColumn::Made(MadeColumn(1)),
                row: reg(36),
                out: reg(37),
            },
            FillStage::Keep {
                test: TestOperation::Less,
                left: of(37),
                right: of(30),
            },
            FillStage::Call {
                meaning: CallMeaning::Lower,
                arguments: operands([of(37)]),
                outputs: regs([38]),
            },
            FillStage::Render {
                node: reg(30),
                context: RenderContext::AssignmentTarget,
                out: reg(39),
            },
            FillStage::Walk {
                relation: node,
                walk: TreeWalk::Axis(AxisDirection::Ancestor),
                start: reg(30),
                out: reg(40),
            },
            FillStage::Walk {
                relation: node,
                walk: TreeWalk::Children(Some(FieldCode(6))),
                start: reg(40),
                out: reg(41),
            },
        ]),
        row: operands([of(30), of(40)]),
    }
}
