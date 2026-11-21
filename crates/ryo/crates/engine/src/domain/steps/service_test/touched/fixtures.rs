//! The steps and stages the traversal cases name. Every register operand has its own number: an operand read is 1 to 3, an
//! output 10 or 11, a register set for a body 12 or 13, a memo key 20 or 21, and a register only a body's own step names 30.

use crate::domain::steps::stages::types::{FillStage, ProbeRange, TreeWalk, UseMode};
use crate::domain::steps::types::{
    AnchorKinds, AxisDirection, BoundKind, CallMeaning, ColumnMask, DefinitionId, EmitTarget,
    MadeColumn, MadeTableId, MatchColumns, NearestEdges, PlacedStep, Polarity, ProgramStep,
    RangeBound, ReadColumn, Register, RegisterList, StepBody, StepMemo, StepOperand, StepOperands,
    StepProgram, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::{KindBits, Relation};
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The step or stage a row names.
#[derive(Clone, Copy, Debug)]
pub(super) enum Scenario {
    /// A step, by its instruction.
    Step(Step),
    /// A stage, by its kind.
    Stage(Stage),
}

/// The steps of the cases: one for each instruction, and one more where an operand may be absent or of another form.
#[derive(Clone, Copy, Debug)]
pub(super) enum Step {
    Kind,
    Match,
    Range,
    RangeOpen,
    Row,
    RowDefinition,
    Children,
    Axis,
    Within,
    NearestTree,
    NearestDerived,
    Closure,
    Read,
    Test,
    Call,
    Render,
    Let,
    Exists,
    ExistsInline,
    Or,
    Fork,
    Yield,
    Latest,
    Emit,
}

/// The stages of the cases: one for each stage, and a probe with no range.
#[derive(Clone, Copy, Debug)]
pub(super) enum Stage {
    Scan,
    Probe,
    ProbePresent,
    Product,
    Use,
    Latest,
    Read,
    Keep,
    Call,
    Render,
    Walk,
}

/// The definition the cases name.
const DEFINITION: DefinitionId = DefinitionId(7);
/// Another.
const OTHER: DefinitionId = DefinitionId(8);

fn reg(number: u32) -> StepOperand {
    StepOperand::Register(Register(number))
}

fn word() -> StepOperand {
    StepOperand::Word(CellWord(5))
}

fn operands<const N: usize>(operands: [StepOperand; N]) -> StepOperands {
    StepOperands(Box::new(operands))
}

fn registers<const N: usize>(numbers: [u32; N]) -> RegisterList {
    RegisterList(Box::new(numbers.map(Register)))
}

fn columns() -> MatchColumns {
    MatchColumns(Box::new([ReadColumn::Made(MadeColumn(0))]))
}

fn bound(value: StepOperand) -> RangeBound {
    RangeBound {
        value,
        end: BoundKind::Inclusive,
    }
}

/// An inline body of one step, which reads register 30.
fn inline() -> StepProgram {
    StepProgram(Box::new([PlacedStep {
        point: None,
        step: ProgramStep::Yield {
            row: registers([30]),
        },
    }]))
}

/// A keyed use of `definition`, binding the value of `register`.
fn used(definition: DefinitionId, register: u32) -> StepBody {
    StepBody::Definition {
        definition,
        bound: ColumnMask(Box::new([MadeColumn(0)])),
        collected: ColumnMask(Box::new([MadeColumn(1)])),
        values: operands([reg(register)]),
    }
}

fn nearest(edges: NearestEdges, condition: StepBody, memo: StepMemo) -> ProgramStep {
    ProgramStep::Nearest {
        edges,
        condition,
        element: Register(12),
        start: Register(1),
        out: Register(10),
        memo,
    }
}

/// The step a row names.
pub(super) fn step(step: Step) -> ProgramStep {
    let made = StepRows::Made(MadeTableId(0));
    let out = Register(10);
    match step {
        Step::Kind => ProgramStep::EachKind {
            relation: Relation::SyntaxTree,
            kinds: AnchorKinds(Box::new([KindBits(1)])),
            out,
        },
        Step::Match => ProgramStep::EachMatch {
            source: StepRows::Definition(DEFINITION),
            columns: columns(),
            values: operands([reg(1), word(), reg(2)]),
            out,
        },
        Step::Range => ProgramStep::EachRange {
            source: made,
            columns: columns(),
            values: operands([reg(1)]),
            range: ReadColumn::Made(MadeColumn(1)),
            low: Some(bound(reg(2))),
            high: Some(bound(reg(3))),
            out,
            range_out: Some(Register(11)),
        },
        Step::RangeOpen => ProgramStep::EachRange {
            source: made,
            columns: columns(),
            values: operands([reg(1)]),
            range: ReadColumn::Made(MadeColumn(1)),
            low: None,
            high: Some(bound(word())),
            out,
            range_out: None,
        },
        Step::Row => ProgramStep::EachRow {
            source: StepRows::Filled(Relation::Function),
            out,
        },
        Step::RowDefinition => ProgramStep::EachRow {
            source: StepRows::Definition(DEFINITION),
            out,
        },
        Step::Children => ProgramStep::Children {
            relation: Relation::SyntaxTree,
            parent: Register(1),
            field: None,
            out,
        },
        Step::Axis => ProgramStep::Axis {
            relation: Relation::SyntaxTree,
            direction: AxisDirection::Ancestor,
            start: Register(1),
            out,
        },
        Step::Within => ProgramStep::Within {
            relation: Relation::SyntaxTree,
            ancestor: Register(1),
            descendant: Register(2),
        },
        Step::NearestTree => nearest(
            NearestEdges::Tree(Relation::SyntaxTree),
            StepBody::Inline(inline()),
            StepMemo::Keyed(registers([20, 21])),
        ),
        Step::NearestDerived => nearest(
            NearestEdges::Derived {
                edges: used(DEFINITION, 2),
                from: Register(13),
            },
            used(OTHER, 3),
            StepMemo::None,
        ),
        Step::Closure => ProgramStep::Closure {
            edges: StepBody::Inline(inline()),
            from: Register(13),
            seed: Register(1),
            out,
        },
        Step::Read => ProgramStep::Read {
            source: StepRows::Definition(DEFINITION),
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(1),
            out,
        },
        Step::Test => ProgramStep::Test {
            test: TestOperation::Equal,
            left: reg(1),
            right: reg(2),
        },
        Step::Call => ProgramStep::Call {
            meaning: CallMeaning::Concat,
            arguments: operands([reg(1), StepOperand::File, reg(2)]),
            outputs: registers([10, 11]),
        },
        Step::Render => ProgramStep::Render {
            node: Register(1),
            context: RenderContext::AssignmentTarget,
            out,
        },
        Step::Let => ProgramStep::Let { from: reg(1), out },
        Step::Exists => ProgramStep::Exists {
            polarity: Polarity::Absent,
            body: used(DEFINITION, 1),
            memo: StepMemo::Keyed(registers([20])),
        },
        Step::ExistsInline => ProgramStep::Exists {
            polarity: Polarity::Present,
            body: StepBody::Inline(inline()),
            memo: StepMemo::None,
        },
        Step::Or => ProgramStep::Or {
            bodies: Box::new([StepBody::Inline(inline()), used(DEFINITION, 1)]),
            outputs: registers([10, 11]),
            memo: StepMemo::Keyed(registers([20])),
        },
        Step::Fork => ProgramStep::Fork {
            slots: Box::new([inline(), inline()]),
        },
        Step::Yield => ProgramStep::Yield {
            row: registers([1, 2]),
        },
        Step::Latest => ProgramStep::Latest {
            table: MadeTableId(0),
            key: operands([reg(1)]),
            bound: reg(2),
            side: OrderBound::Before,
            out,
        },
        Step::Emit => ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: operands([reg(1), word(), reg(2)]),
        },
    }
}

/// The stage a row names.
pub(super) fn stage(stage: Stage) -> FillStage {
    let filled = StepRows::Filled(Relation::Function);
    let out = Register(10);
    match stage {
        Stage::Scan => FillStage::Scan {
            source: StepRows::Definition(DEFINITION),
            columns: columns(),
            values: operands([word()]),
            out,
        },
        Stage::Probe => FillStage::Probe {
            source: filled,
            columns: columns(),
            values: operands([reg(1)]),
            range: Some(ProbeRange {
                column: ReadColumn::Made(MadeColumn(1)),
                low: bound(reg(2)),
                high: bound(reg(3)),
            }),
            mode: UseMode::Rows,
            out,
        },
        Stage::ProbePresent => FillStage::Probe {
            source: filled,
            columns: columns(),
            values: operands([reg(1)]),
            range: None,
            mode: UseMode::Present,
            out,
        },
        Stage::Product => FillStage::Product {
            source: StepRows::Definition(DEFINITION),
            out,
        },
        Stage::Use => FillStage::Use {
            definition: DEFINITION,
            bound: ColumnMask(Box::new([MadeColumn(0), MadeColumn(1)])),
            collected: ColumnMask(Box::new([MadeColumn(2), MadeColumn(3)])),
            values: operands([reg(1), reg(2)]),
            mode: UseMode::Rows,
            outputs: registers([10, 11]),
        },
        Stage::Latest => FillStage::Latest {
            table: MadeTableId(0),
            key: operands([reg(1)]),
            bound: reg(2),
            side: OrderBound::Before,
            out,
        },
        Stage::Read => FillStage::Read {
            source: filled,
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(1),
            out,
        },
        Stage::Keep => FillStage::Keep {
            test: TestOperation::Less,
            left: reg(1),
            right: reg(2),
        },
        Stage::Call => FillStage::Call {
            meaning: CallMeaning::Concat,
            arguments: operands([reg(1), reg(2)]),
            outputs: registers([10]),
        },
        Stage::Render => FillStage::Render {
            node: Register(1),
            context: RenderContext::AssignmentTarget,
            out,
        },
        Stage::Walk => FillStage::Walk {
            relation: Relation::SyntaxTree,
            walk: TreeWalk::Axis(AxisDirection::Descendant),
            start: Register(1),
            out,
        },
    }
}
