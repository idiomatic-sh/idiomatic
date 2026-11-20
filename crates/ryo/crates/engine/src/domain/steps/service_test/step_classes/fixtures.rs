//! The steps the cost class cases name, each the smallest of its kind.

use crate::domain::steps::types::{
    AnchorKinds, AxisDirection, BoundKind, CallMeaning, ColumnMask, DefinitionId, EmitTarget,
    MadeColumn, MadeTableId, MatchColumns, NearestEdges, Polarity, ProgramStep, RangeBound,
    ReadColumn, Register, RegisterList, StepBody, StepMemo, StepOperand, StepOperands, StepProgram,
    StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::{KindBits, NodeColumn, Relation};
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// The step a row classes.
#[derive(Clone, Copy, Debug)]
pub(super) enum Scenario {
    /// `each kind is the files`.
    Kind,
    /// `each match on a register is a keys fanout`.
    MatchRegister,
    /// `each match on constants alone is the files`.
    MatchConstants,
    /// `each match over a written table is a fanout`.
    MatchWritten,
    /// `each range with both bounds registers is a keys fanout`.
    RangeRegisters,
    /// `each range on constants alone is the files`.
    RangeConstants,
    /// `each row of a relation is the files`.
    RowRelation,
    /// `each row of a definitions table is the files`.
    RowDefinition,
    /// `each row of a written table is a fanout`.
    RowWritten,
    /// `children are a fanout`.
    Children,
    /// `the parent axis is constant`.
    AxisParent,
    /// `the descendant axis is a fanout`.
    AxisDescendant,
    /// `within is a fanout`.
    Within,
    /// `nearest is a fanout`.
    Nearest,
    /// `nearest over edges that collect rows with nothing bound is the files`.
    NearestUnbound,
    /// `closure is a fanout`.
    Closure,
    /// `read is constant`.
    Read,
    /// `test is constant`.
    Test,
    /// `call is constant`.
    Call,
    /// `render is constant`.
    Render,
    /// `let is constant`.
    Let,
    /// `exists over an inline body is constant`.
    ExistsInline,
    /// `exists over a use that binds nothing is constant`.
    ExistsUnbound,
    /// `exists over a use that binds a column is a fanout`.
    ExistsBound,
    /// `or over inline bodies is constant`.
    OrInline,
    /// `or over a use that binds a column is a fanout`.
    OrBound,
    /// `or over a use that binds nothing is the files`.
    OrUnbound,
    /// `fork is constant`.
    Fork,
    /// `yield is constant`.
    Yield,
    /// `latest is a fanout`.
    Latest,
    /// `emit is constant`.
    Emit,
}

/// A register.
const fn register(number: u32) -> Register {
    Register(number)
}

/// StepOperands that are one register's value.
fn keyed() -> StepOperands {
    StepOperands(Box::new([StepOperand::Register(register(0))]))
}

/// StepOperands that are one constant.
fn constant() -> StepOperands {
    StepOperands(Box::new([StepOperand::Word(CellWord(7))]))
}

/// The one column a match is on.
fn column() -> MatchColumns {
    MatchColumns(Box::new([ReadColumn::Made(MadeColumn(0))]))
}

/// A bound that is `value`.
fn bound(value: StepOperand) -> Option<RangeBound> {
    Some(RangeBound {
        value,
        end: BoundKind::Inclusive,
    })
}

/// A use of a definition that binds `bound` columns and collects `collected`.
fn used(bound: &[u16], collected: &[u16]) -> StepBody {
    let mask = |columns: &[u16]| ColumnMask(columns.iter().copied().map(MadeColumn).collect());
    StepBody::Definition {
        definition: DefinitionId(0),
        bound: mask(bound),
        collected: mask(collected),
        values: StepOperands(
            bound
                .iter()
                .map(|_| StepOperand::Register(register(0)))
                .collect(),
        ),
    }
}

/// An inline body of no step.
fn inline() -> StepBody {
    StepBody::Inline(StepProgram::default())
}

/// A range over a made table with `values` and both bounds `end`.
fn range(values: StepOperands, end: &StepOperand) -> ProgramStep {
    ProgramStep::EachRange {
        source: StepRows::Made(MadeTableId(0)),
        columns: column(),
        values,
        range: ReadColumn::Made(MadeColumn(1)),
        low: bound(end.clone()),
        high: bound(end.clone()),
        out: register(1),
        range_out: None,
    }
}

/// A match over `source` on `values`.
fn matched(source: StepRows, values: StepOperands) -> ProgramStep {
    ProgramStep::EachMatch {
        source,
        columns: column(),
        values,
        out: register(1),
    }
}

/// An `Or` over `body`.
fn or(body: StepBody) -> ProgramStep {
    ProgramStep::Or {
        bodies: Box::new([body]),
        outputs: RegisterList::default(),
        memo: StepMemo::None,
    }
}

/// An `Exists` over `body`.
fn exists(body: StepBody) -> ProgramStep {
    ProgramStep::Exists {
        polarity: Polarity::Present,
        body,
        memo: StepMemo::None,
    }
}

/// The edges of the nearest walk a row names: derived edges that collect rows with nothing bound, or the tree.
fn nearest_edges(scenario: Scenario) -> NearestEdges {
    if matches!(scenario, Scenario::NearestUnbound) {
        NearestEdges::Derived {
            edges: used(&[], &[0]),
            from: register(2),
        }
    } else {
        NearestEdges::Tree
    }
}

/// The step a row names.
pub(super) fn step(scenario: Scenario) -> ProgramStep {
    let made = StepRows::Made(MadeTableId(0));
    match scenario {
        Scenario::Kind => ProgramStep::EachKind {
            relation: Relation::SyntaxTree,
            kinds: AnchorKinds(Box::new([KindBits(1)])),
            out: register(0),
        },
        Scenario::MatchRegister => matched(made, keyed()),
        Scenario::MatchConstants => matched(made, constant()),
        Scenario::MatchWritten => matched(StepRows::Written(MadeTableId(0)), constant()),
        Scenario::RangeRegisters => range(constant(), &StepOperand::Register(register(0))),
        Scenario::RangeConstants => range(constant(), &StepOperand::Word(CellWord(1))),
        Scenario::RowRelation => ProgramStep::EachRow {
            source: StepRows::Filled(Relation::Function),
            out: register(0),
        },
        Scenario::RowDefinition => ProgramStep::EachRow {
            source: StepRows::Definition(DefinitionId(0)),
            out: register(0),
        },
        Scenario::RowWritten => ProgramStep::EachRow {
            source: StepRows::Written(MadeTableId(0)),
            out: register(0),
        },
        Scenario::Children => ProgramStep::Children {
            parent: register(0),
            field: None,
            out: register(1),
        },
        Scenario::AxisParent => ProgramStep::Axis {
            direction: AxisDirection::Parent,
            start: register(0),
            out: register(1),
        },
        Scenario::AxisDescendant => ProgramStep::Axis {
            direction: AxisDirection::Descendant,
            start: register(0),
            out: register(1),
        },
        Scenario::Within => ProgramStep::Within {
            ancestor: register(0),
            descendant: register(1),
        },
        Scenario::Nearest | Scenario::NearestUnbound => ProgramStep::Nearest {
            edges: nearest_edges(scenario),
            condition: inline(),
            element: register(0),
            start: register(1),
            out: register(2),
            memo: StepMemo::None,
        },
        Scenario::Closure => ProgramStep::Closure {
            edges: used(&[0], &[1]),
            from: register(0),
            seed: register(1),
            out: register(2),
        },
        Scenario::Read => ProgramStep::Read {
            source: StepRows::Node(Relation::SyntaxTree),
            column: ReadColumn::Node(NodeColumn::Kind),
            row: register(0),
            out: register(1),
        },
        Scenario::Test => ProgramStep::Test {
            test: TestOperation::Equal,
            left: StepOperand::Register(register(0)),
            right: StepOperand::Word(CellWord(1)),
        },
        Scenario::Call => ProgramStep::Call {
            meaning: CallMeaning::Words,
            arguments: keyed(),
            outputs: RegisterList::default(),
        },
        Scenario::Render => ProgramStep::Render {
            node: register(0),
            context: RenderContext::AssignmentTarget,
            out: register(1),
        },
        Scenario::Let => ProgramStep::Let {
            from: StepOperand::File,
            out: register(0),
        },
        Scenario::ExistsInline => exists(inline()),
        Scenario::ExistsUnbound => exists(used(&[], &[])),
        Scenario::ExistsBound => exists(used(&[0], &[])),
        Scenario::OrInline => or(inline()),
        Scenario::OrBound => or(used(&[0], &[1])),
        Scenario::OrUnbound => or(used(&[], &[0])),
        Scenario::Fork => ProgramStep::Fork {
            slots: Box::new([StepProgram::default()]),
        },
        Scenario::Yield => ProgramStep::Yield {
            row: RegisterList::default(),
        },
        Scenario::Latest => ProgramStep::Latest {
            table: MadeTableId(0),
            key: keyed(),
            bound: StepOperand::Register(register(1)),
            side: OrderBound::Before,
            out: register(2),
        },
        Scenario::Emit => ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: keyed(),
        },
    }
}
