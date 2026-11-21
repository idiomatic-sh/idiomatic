//! The cost class of each stage, with the ways a probe's and a use's operands change it, and of each per-row meaning.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::steps::service::{call_class, stage_class};
use crate::domain::steps::stages::types::{FillStage, ProbeRange, TreeWalk, UseMode};
use crate::domain::steps::types::{
    AxisDirection, BoundKind, CallMeaning, ColumnMask, CostClass, DefinitionId, MadeColumn,
    MadeTableId, MatchColumns, RangeBound, ReadColumn, RegexId, Register, RegisterList,
    StepOperand, StepOperands, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::semantics::types::RenderContext;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_orders::types::OrderBound;

/// What a row classes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// `a scan is the files`.
    Scan,
    /// `a probe by a register is a keys fanout`.
    ProbeRegister,
    /// `a probe with no register is file`.
    ProbeConstants,
    /// `a probe with no register that asks present is constant`.
    ProbePresent,
    /// `a probe ranged by two registers is a keys fanout`.
    ProbeRanged,
    /// `a product is the files`.
    Product,
    /// `a use that binds a column is a keys fanout`.
    UseBound,
    /// `an empty mask in rows mode is file`.
    UseUnboundRows,
    /// `a use that binds nothing and asks absent is constant`.
    UseUnboundAbsent,
    /// `a stage latest is a fanout`.
    Latest,
    /// `a stage read is constant`.
    Read,
    /// `a keep is constant`.
    Keep,
    /// `a stage call is constant`.
    Call,
    /// `a stage render is constant`.
    Render,
    /// `a walk is a fanout`.
    Walk,
    /// A per-row meaning.
    Meaning(CallMeaning),
}

type Input = Scenario;
type Value = CostClass;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Scan,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("a_probe_by_a_register_is_a_keys_fanout"),
        word: Word::Succeeds,
        input: || Scenario::ProbeRegister,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("a_probe_with_no_register_is_file"),
        word: Word::Succeeds,
        input: || Scenario::ProbeConstants,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("a_probe_with_no_register_that_asks_present_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::ProbePresent,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("a_probe_ranged_by_two_registers_is_a_keys_fanout"),
        word: Word::Succeeds,
        input: || Scenario::ProbeRanged,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("a_product_is_the_files"),
        word: Word::Succeeds,
        input: || Scenario::Product,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("a_use_that_binds_a_column_is_a_keys_fanout"),
        word: Word::Succeeds,
        input: || Scenario::UseBound,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("an_empty_mask_in_rows_mode_is_file"),
        word: Word::Succeeds,
        input: || Scenario::UseUnboundRows,
        expect: Expect::Value(CostClass::File),
    },
    Case {
        name: CaseName("a_use_that_binds_nothing_and_asks_absent_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::UseUnboundAbsent,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("a_stage_latest_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::Latest,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("a_stage_read_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Read,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("a_keep_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Keep,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("a_stage_call_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Call,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("a_stage_render_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Render,
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("a_walk_is_a_fanout"),
        word: Word::Succeeds,
        input: || Scenario::Walk,
        expect: Expect::Value(CostClass::Fanout),
    },
    Case {
        name: CaseName("the_meaning_span_between_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::SpanBetween),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_span_bounds_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::SpanBounds),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_source_matches_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::SourceMatches(RegexId(0))),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_capture_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::Capture),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_replace_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::Replace(RegexId(0))),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_words_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::Words),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_rank_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::Rank),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_concat_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::Concat),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_lower_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::Lower),
        expect: Expect::Value(CostClass::Constant),
    },
    Case {
        name: CaseName("the_meaning_fits_rendered_is_constant"),
        word: Word::Succeeds,
        input: || Scenario::Meaning(CallMeaning::FitsRendered),
        expect: Expect::Value(CostClass::Constant),
    },
];

/// StepOperands that are one register's value.
fn keyed() -> StepOperands {
    StepOperands(Box::new([StepOperand::Register(Register(0))]))
}

/// StepOperands that are one constant.
fn constant() -> StepOperands {
    StepOperands(Box::new([StepOperand::Word(CellWord(7))]))
}

/// A probe of a made table on `values`, held to `range`, in `mode`.
fn probe(values: StepOperands, range: Option<ProbeRange>, mode: UseMode) -> FillStage {
    FillStage::Probe {
        source: StepRows::Made(MadeTableId(0)),
        columns: MatchColumns(Box::new([ReadColumn::Made(MadeColumn(0))])),
        values,
        range,
        mode,
        out: Register(1),
    }
}

/// A range whose two ends are registers.
fn keyed_range() -> ProbeRange {
    let end = || RangeBound {
        value: StepOperand::Register(Register(2)),
        end: BoundKind::Strict,
    };
    ProbeRange {
        column: ReadColumn::Made(MadeColumn(1)),
        low: end(),
        high: end(),
    }
}

/// A use of a definition that binds `bound` columns, in `mode`.
fn used(bound: &[u16], mode: UseMode) -> FillStage {
    FillStage::Use {
        definition: DefinitionId(0),
        bound: ColumnMask(bound.iter().copied().map(MadeColumn).collect()),
        collected: ColumnMask(Box::new([MadeColumn(1)])),
        values: StepOperands(
            bound
                .iter()
                .map(|_| StepOperand::Register(Register(0)))
                .collect(),
        ),
        mode,
        outputs: RegisterList(Box::new([Register(3)])),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let made = StepRows::Made(MadeTableId(0));
    let stage = match scenario {
        Scenario::Meaning(meaning) => return Ok(call_class(meaning)),
        Scenario::Scan => FillStage::Scan {
            source: made,
            columns: MatchColumns::default(),
            values: StepOperands::default(),
            out: Register(0),
        },
        Scenario::ProbeRegister => probe(keyed(), None, UseMode::Rows),
        Scenario::ProbeConstants => probe(constant(), None, UseMode::Rows),
        Scenario::ProbePresent => probe(constant(), None, UseMode::Present),
        Scenario::ProbeRanged => probe(constant(), Some(keyed_range()), UseMode::Rows),
        Scenario::Product => FillStage::Product {
            source: made,
            out: Register(0),
        },
        Scenario::UseBound => used(&[0], UseMode::Rows),
        Scenario::UseUnboundRows => used(&[], UseMode::Rows),
        Scenario::UseUnboundAbsent => used(&[], UseMode::Absent),
        Scenario::Latest => FillStage::Latest {
            table: MadeTableId(0),
            key: keyed(),
            bound: StepOperand::Register(Register(1)),
            side: OrderBound::Before,
            out: Register(2),
        },
        Scenario::Read => FillStage::Read {
            source: made,
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(0),
            out: Register(1),
        },
        Scenario::Keep => FillStage::Keep {
            test: TestOperation::Equal,
            left: StepOperand::Register(Register(0)),
            right: StepOperand::Word(CellWord(1)),
        },
        Scenario::Call => FillStage::Call {
            meaning: CallMeaning::Lower,
            arguments: keyed(),
            outputs: RegisterList::default(),
        },
        Scenario::Render => FillStage::Render {
            node: Register(0),
            context: RenderContext::AttributeReceiver,
            out: Register(1),
        },
        Scenario::Walk => FillStage::Walk {
            relation: Relation::SyntaxTree,
            walk: TreeWalk::Axis(AxisDirection::Ancestor),
            start: Register(0),
            out: Register(1),
        },
    };
    Ok(stage_class(&stage))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
