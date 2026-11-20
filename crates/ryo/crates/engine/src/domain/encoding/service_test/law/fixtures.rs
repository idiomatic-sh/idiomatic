//! The pack of the encoding law: one rule whose branch holds every step, over two definitions (one a pattern in call form, one
//! a pattern in table form) and four made tables, one of whose pipelines holds every stage.

use super::super::fixtures::{
    every_stage, mask, of, operands, pack, program, reg, regs, rule, stages, word,
};
use crate::domain::lowered::types::{
    ConstantRows, DefinitionPattern, Emptiness, FillInput, GroupFill, LoweredPack, MadeFill,
    MadeTable, OrderedFill, PackDefinition, PatternForm, RowValue,
};
use crate::domain::steps::stages::types::{FillStage, Pipeline};
use crate::domain::steps::types::{
    AnchorKinds, AxisDirection, BoundKind, CallMeaning, ColumnList, DefinitionId, EmitTarget,
    MadeColumn, MadeTableId, MatchColumns, NearestEdges, Polarity, ProgramStep, RangeBound,
    ReadColumn, RegexId, StepBody, StepMemo, StepOperand, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::FieldCode;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::collections::relations::syntax_error::item::SyntaxErrorColumn;
use ryo_relations::domain::relations::types::{Column, KindBits, NodeColumn, Relation};
use ryo_values::domain::plan_orders::types::OrderBound;
use ryo_values::domain::plan_parts::types::{InputNumber, RepeatedRows};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// The filled relation the law's steps read, and one of its columns.
const FILLED: Relation = Relation::SyntaxError;
/// The column of it the law's steps match and read.
const FILLED_COLUMN: Column = Column::SyntaxError(SyntaxErrorColumn::File);

/// A made column as a column a step reads.
const fn made(column: u16) -> ReadColumn {
    ReadColumn::Made(MadeColumn(column))
}

/// The columns a match is on.
fn columns<const N: usize>(each: [ReadColumn; N]) -> MatchColumns {
    MatchColumns(Box::new(each))
}

/// A use of the definition in call form: its first column bound to register 1, its second collected.
fn call_use() -> StepBody {
    StepBody::Definition {
        definition: DefinitionId(0),
        bound: mask([0]),
        collected: mask([1]),
        values: operands([of(1)]),
    }
}

/// A use of the definition in table form: its first column bound to register 1, its second collected.
fn table_use() -> StepBody {
    StepBody::Definition {
        definition: DefinitionId(1),
        bound: mask([0]),
        collected: mask([1]),
        values: operands([of(1)]),
    }
}

/// A use of the definition in table form that asks only whether a row is there: its first column bound, nothing collected.
fn table_presence() -> StepBody {
    StepBody::Definition {
        definition: DefinitionId(1),
        bound: mask([0]),
        collected: mask([]),
        values: operands([of(1)]),
    }
}

/// The rule's branch: every step, the anchor first; no other step is file-sized.
fn every_step() -> Vec<ProgramStep> {
    let condition = StepBody::Inline(program(vec![
        ProgramStep::Test {
            test: TestOperation::Matches(RegexId(4)),
            left: of(20),
            right: word(9),
        },
        ProgramStep::Yield { row: regs([]) },
    ]));
    vec![
        ProgramStep::EachKind {
            kinds: AnchorKinds(Box::new([KindBits(2), KindBits(3)])),
            out: reg(0),
        },
        ProgramStep::EachMatch {
            source: StepRows::Filled(FILLED),
            columns: columns([ReadColumn::Filled(FILLED_COLUMN)]),
            values: operands([of(0)]),
            out: reg(1),
        },
        ProgramStep::EachMatch {
            source: StepRows::Made(MadeTableId(0)),
            columns: columns([made(0)]),
            values: operands([of(1)]),
            out: reg(16),
        },
        ProgramStep::EachRange {
            source: StepRows::Made(MadeTableId(1)),
            columns: columns([made(0)]),
            values: operands([of(1)]),
            range: made(1),
            low: Some(RangeBound {
                value: of(0),
                end: BoundKind::Inclusive,
            }),
            high: None,
            out: reg(2),
            range_out: Some(reg(3)),
        },
        ProgramStep::EachRow {
            source: StepRows::Written(MadeTableId(3)),
            out: reg(4),
        },
        ProgramStep::Children {
            parent: reg(0),
            field: Some(FieldCode(6)),
            out: reg(5),
        },
        ProgramStep::Axis {
            direction: AxisDirection::NextSibling,
            start: reg(5),
            out: reg(6),
        },
        ProgramStep::Within {
            ancestor: reg(0),
            descendant: reg(6),
        },
        ProgramStep::Nearest {
            edges: NearestEdges::Tree,
            condition,
            element: reg(20),
            start: reg(6),
            out: reg(7),
            memo: StepMemo::Keyed(regs([6])),
        },
        ProgramStep::Closure {
            edges: call_use(),
            from: reg(1),
            seed: reg(7),
            out: reg(8),
        },
        ProgramStep::Read {
            source: StepRows::Node(Relation::SyntaxTree),
            column: ReadColumn::Node(NodeColumn::Kind),
            row: reg(8),
            out: reg(9),
        },
        ProgramStep::Test {
            test: TestOperation::In,
            left: of(9),
            right: StepOperand::Words(Box::new([CellWord(12), CellWord(11)])),
        },
        ProgramStep::Call {
            meaning: CallMeaning::Replace(RegexId(2)),
            arguments: operands([of(9), word(11)]),
            outputs: regs([10]),
        },
        ProgramStep::Render {
            node: reg(8),
            context: RenderContext::PositionalArgument,
            out: reg(11),
        },
        ProgramStep::Let {
            from: StepOperand::File,
            out: reg(12),
        },
        ProgramStep::Exists {
            polarity: Polarity::Absent,
            body: table_presence(),
            memo: StepMemo::None,
        },
        ProgramStep::Or {
            bodies: Box::new([
                StepBody::Inline(program(vec![ProgramStep::Yield { row: regs([13]) }])),
                call_use(),
                table_use(),
            ]),
            outputs: regs([14]),
            memo: StepMemo::Keyed(regs([])),
        },
        ProgramStep::Latest {
            table: MadeTableId(2),
            key: operands([of(1)]),
            bound: of(9),
            side: OrderBound::AtOrBefore,
            out: reg(15),
        },
        ProgramStep::Emit {
            target: EmitTarget::Frontier(SelectedRuleOrdinal(0), InputNumber(1)),
            columns: operands([of(14), StepOperand::File]),
        },
        ProgramStep::Fork {
            slots: Box::new([program(vec![ProgramStep::Emit {
                target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
                columns: operands([of(15)]),
            }])]),
        },
    ]
}

/// One pipeline that scans `source`, of rows of two cells.
fn scanned(source: StepRows, bucket: Option<StepOperand>) -> Pipeline {
    let (columns, values) = match bucket {
        Some(value) => (
            columns([ReadColumn::Filled(FILLED_COLUMN)]),
            operands([value]),
        ),
        None => (MatchColumns::default(), operands([])),
    };
    Pipeline {
        stages: stages(vec![FillStage::Scan {
            source,
            columns,
            values,
            out: reg(50),
        }]),
        row: operands([of(50), of(50)]),
    }
}

/// A made table of two columns.
fn table(node: u8, fill: MadeFill, emptiness: Emptiness) -> MadeTable {
    MadeTable {
        node: NodeDigest([node; 16]),
        fill,
        width: ColumnCount(2),
        indexes: vec![ColumnList(Box::new([MadeColumn(0)]))],
        emptiness,
    }
}

/// A definition of two columns whose first pattern binds the first and collects the second.
fn definition(
    node: u8,
    formals: [u32; 2],
    keyed: PatternForm,
    own: MadeTableId,
    emptiness: Emptiness,
) -> PackDefinition {
    PackDefinition {
        node: NodeDigest([node; 16]),
        formals: regs(formals),
        emptiness,
        patterns: vec![
            DefinitionPattern {
                bound: mask([0]),
                collected: mask([1]),
                form: keyed,
            },
            DefinitionPattern {
                bound: mask([]),
                collected: mask([0, 1]),
                form: PatternForm::Table(own),
            },
            DefinitionPattern {
                bound: mask([0]),
                collected: mask([]),
                form: PatternForm::Table(own),
            },
        ],
    }
}

/// The law's pack.
pub(super) fn every_variant() -> LoweredPack {
    let called = program(vec![
        ProgramStep::Children {
            parent: reg(60),
            field: None,
            out: reg(61),
        },
        ProgramStep::Yield { row: regs([61]) },
    ]);
    let input = |pipeline, order| FillInput {
        pipelines: Box::new([pipeline]),
        keys: ColumnList(Box::new([MadeColumn(0)])),
        order,
    };
    let row = Emptiness::Row(
        FILLED,
        Some(RowValue {
            column: FILLED_COLUMN,
            value: CellWord(5),
        }),
    );
    let mut law = pack(vec![rule(0, every_step())]);
    law.definitions = vec![
        definition(
            10,
            [60, 61],
            PatternForm::Call(called),
            MadeTableId(0),
            Emptiness::Kind(AnchorKinds(Box::new([KindBits(2)]))),
        ),
        definition(
            11,
            [62, 63],
            PatternForm::Table(MadeTableId(1)),
            MadeTableId(1),
            Emptiness::Any(Box::new([
                Emptiness::Definition(DefinitionId(0)),
                Emptiness::Unknown,
            ])),
        ),
    ];
    law.tables = vec![
        table(
            20,
            MadeFill::Rows(Box::new([every_stage()]), RepeatedRows::Dropped),
            Emptiness::All(Box::new([Emptiness::Definition(DefinitionId(1)), row])),
        ),
        table(
            21,
            MadeFill::Group(
                GroupFill::Count {
                    distinct: Some(MadeColumn(1)),
                },
                input(scanned(StepRows::Filled(FILLED), Some(word(5))), None),
            ),
            Emptiness::Table(MadeTableId(3)),
        ),
        table(
            22,
            MadeFill::Ordered(OrderedFill::Sorted(input(
                scanned(StepRows::Made(MadeTableId(1)), None),
                Some(MadeColumn(1)),
            ))),
            Emptiness::Table(MadeTableId(1)),
        ),
        table(
            23,
            MadeFill::Constant(ConstantRows(vec![Box::new([CellWord(5), CellWord(40)])])),
            Emptiness::All(Box::new([])),
        ),
    ];
    law
}
