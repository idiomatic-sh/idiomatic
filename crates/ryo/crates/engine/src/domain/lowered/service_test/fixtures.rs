//! What the cases of the pack's walks share: a pack that holds a definition, a rule and a table of every fill, and a pack
//! whose one rule holds every step that runs bodies. Every step and stage writes registers of its own, so the registers
//! written, in the order met, say what a walk met and when. And what the cases of the graph tables share: the columns
//! each small input's fill is told, and an input of rows of a width.

use crate::domain::code::records::types::{EdgeColumns, VisitColumns, WorklistColumns};
use crate::domain::code::types::ShapeChoice;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, FillInput, GraphFill, GroupFill,
    LoweredPack, LoweredRule, MadeFill, MadeTable, OrderedFill, PackDefinition, PatternForm,
};
use crate::domain::steps::stages::types::{FillStage, Pipeline, PlacedStage};
use crate::domain::steps::types::{
    ColumnList, ColumnMask, DefinitionId, MadeColumn, MadeTableId, NearestEdges, PlacedStep,
    Polarity, ProgramStep, Register, RegisterList, StepBody, StepMemo, StepOperand, StepOperands,
    StepProgram, StepRows,
};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

fn set(out: u32) -> ProgramStep {
    ProgramStep::Let {
        from: StepOperand::File,
        out: Register(out),
    }
}

fn program<const N: usize>(steps: [ProgramStep; N]) -> StepProgram {
    StepProgram(Box::new(steps.map(|step| PlacedStep { point: None, step })))
}

/// A pipeline of one stage for each register, which the stage writes.
fn pipeline<const N: usize>(outs: [u32; N]) -> Pipeline {
    let stage = |out| PlacedStage {
        point: None,
        stage: FillStage::Product {
            source: StepRows::Made(MadeTableId(0)),
            out: Register(out),
        },
    };
    Pipeline {
        stages: Box::new(outs.map(stage)),
        row: StepOperands::default(),
    }
}

fn input(out: u32) -> FillInput {
    FillInput {
        pipelines: Box::new([pipeline([out])]),
        keys: ColumnList::default(),
        order: None,
    }
}

fn table(fill: MadeFill) -> MadeTable {
    MadeTable {
        node: NodeDigest::default(),
        fill,
        width: ColumnCount(1),
        indexes: Vec::new(),
        emptiness: Emptiness::Unknown,
    }
}

fn pattern(form: PatternForm) -> DefinitionPattern {
    DefinitionPattern {
        bound: ColumnMask::default(),
        collected: ColumnMask::default(),
        form,
        shape: ShapeChoice::Decided,
    }
}

fn branch(program: StepProgram) -> Branch {
    Branch {
        emptiness: Emptiness::Unknown,
        program,
    }
}

/// A definition of a call form (registers 1 and 2) and a table form; a rule of two branches (3; then 4 with an inline body
/// that sets 5, then 6); and a table of each fill: rows (two pipelines, 10 and 11, then 12), a group (13), a sorted table
/// (14), an ordered match (15 and 16), a graph of two inputs (17 and 18) and written rows.
pub(super) fn whole() -> LoweredPack {
    let definition = PackDefinition {
        node: NodeDigest::default(),
        formals: RegisterList::default(),
        emptiness: Emptiness::Unknown,
        patterns: vec![
            pattern(PatternForm::Call(program([set(1), set(2)]))),
            pattern(PatternForm::Table(MadeTableId(0))),
        ],
    };
    let exists = ProgramStep::Exists {
        polarity: Polarity::Present,
        body: StepBody::Inline(program([set(5)])),
        memo: StepMemo::None,
    };
    let rule = LoweredRule {
        rule: SelectedRuleOrdinal(0),
        node: NodeDigest::default(),
        branches: vec![
            branch(program([set(3)])),
            branch(program([set(4), exists, set(6)])),
        ],
        columns: RuleColumns::default(),
    };
    let rows = MadeFill::Rows(
        Box::new([pipeline([10, 11]), pipeline([12])]),
        RepeatedRows::Dropped,
    );
    LoweredPack {
        definitions: vec![definition],
        rules: vec![rule],
        tables: vec![
            table(rows),
            table(MadeFill::Group(GroupFill::Places, input(13))),
            table(MadeFill::Ordered(OrderedFill::Sorted(input(14)))),
            table(MadeFill::Ordered(OrderedFill::Match(input(15), input(16)))),
            table(MadeFill::Graph(
                GraphFill::Cycles(EdgeColumns {
                    source: MadeColumn(0),
                    destination: MadeColumn(0),
                }),
                Box::new([input(17), input(18)]),
            )),
            table(MadeFill::Constant(ConstantRows::default())),
        ],
        ..LoweredPack::default()
    }
}

/// A body written in place.
fn inline<const N: usize>(steps: [ProgramStep; N]) -> StepBody {
    StepBody::Inline(program(steps))
}

/// One rule of one branch that holds every step that runs bodies, each writing registers of its own: 20; a nearest walk
/// over derived edges (it writes 22, 25 and 26; its edges set 21; its condition sets 23 and holds an `Exists` whose body sets
/// 24); a closure (28 and 29; its edges set 27); an `Or` of two inline bodies (30 and 31) around a use of a definition, which
/// is not gone into (it writes 32); a `Fork` of two slots (33; then 34 and a `Fork` whose slot sets 35); and 36.
pub(super) fn nested() -> LoweredPack {
    let exists = |body| ProgramStep::Exists {
        polarity: Polarity::Present,
        body,
        memo: StepMemo::None,
    };
    let nearest = ProgramStep::Nearest {
        edges: NearestEdges::Derived {
            edges: inline([set(21)]),
            from: Register(22),
        },
        condition: inline([set(23), exists(inline([set(24)]))]),
        element: Register(25),
        start: Register(20),
        out: Register(26),
        memo: StepMemo::None,
    };
    let closure = ProgramStep::Closure {
        edges: inline([set(27)]),
        from: Register(28),
        seed: Register(20),
        out: Register(29),
    };
    let used = StepBody::Definition {
        definition: DefinitionId(0),
        bound: ColumnMask::default(),
        collected: ColumnMask::default(),
        values: StepOperands::default(),
    };
    let or = ProgramStep::Or {
        bodies: Box::new([inline([set(30)]), used, inline([set(31)])]),
        outputs: RegisterList(Box::new([Register(32)])),
        memo: StepMemo::None,
    };
    let fork = ProgramStep::Fork {
        slots: Box::new([
            program([set(33)]),
            program([
                set(34),
                ProgramStep::Fork {
                    slots: Box::new([program([set(35)])]),
                },
            ]),
        ]),
    };
    let rule = LoweredRule {
        rule: SelectedRuleOrdinal(0),
        node: NodeDigest::default(),
        branches: vec![branch(program([
            set(20),
            nearest,
            closure,
            or,
            fork,
            set(36),
        ]))],
        columns: RuleColumns::default(),
    };
    LoweredPack {
        rules: vec![rule],
        ..LoweredPack::default()
    }
}

/// The edge columns of the small `follow`: an edge row is its key, its source, its destination.
pub(super) const EDGES: EdgeColumns = EdgeColumns {
    source: MadeColumn(1),
    destination: MadeColumn(2),
};

/// The columns of the small ordered walk: a root row is its key and its root; an edge row is its key, its source, its
/// destination, its order and its identity.
pub(super) const VISITS: VisitColumns = VisitColumns {
    root: MadeColumn(1),
    source: MadeColumn(1),
    destination: MadeColumn(2),
    edge: MadeColumn(4),
};

/// The columns of the small worklist: a root row is its key, its root and its state; an edge row is its key, its source,
/// its destination, its order and its two states; a lattice row is its two states.
pub(super) const STATES: WorklistColumns = WorklistColumns {
    root: MadeColumn(1),
    state: MadeColumn(2),
    source: MadeColumn(1),
    destination: MadeColumn(2),
    before: MadeColumn(4),
    after: MadeColumn(5),
    lower: MadeColumn(0),
    upper: MadeColumn(1),
};

/// The edge columns of the small `cycles`: an edge row is its source and its destination.
pub(super) const RING: EdgeColumns = EdgeColumns {
    source: MadeColumn(0),
    destination: MadeColumn(1),
};

/// A pipeline of no stage whose rows are `width` wide.
pub(super) fn graph_pipeline(width: u32) -> Pipeline {
    Pipeline {
        stages: Box::new([]),
        row: StepOperands(
            (0..width)
                .map(|at| StepOperand::Register(Register(at)))
                .collect(),
        ),
    }
}

/// An input of one pipeline whose rows are `width` wide, keyed on `keys`, ordered by `order`.
pub(super) fn graph_input(width: u32, keys: &[u16], order: Option<u16>) -> FillInput {
    FillInput {
        pipelines: Box::new([graph_pipeline(width)]),
        keys: ColumnList(keys.iter().copied().map(MadeColumn).collect()),
        order: order.map(MadeColumn),
    }
}
