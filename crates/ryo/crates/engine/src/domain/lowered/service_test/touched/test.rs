//! The walk of what a pack's steps and stages touch: every program and every pipeline of every fill is met, each step and
//! stage once. Every step and stage of the pack a row walks writes a register of its own, so the registers written, in the
//! order met, say what was met and when.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::ShapeChoice;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::service::each_touched;
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, FillInput, GraphFill, GroupFill,
    LoweredPack, LoweredRule, MadeFill, MadeTable, OrderedFill, PackDefinition, PatternForm,
};
use crate::domain::steps::stages::types::{FillStage, Pipeline, PlacedStage};
use crate::domain::steps::types::{
    ColumnList, ColumnMask, MadeTableId, PlacedStep, Polarity, ProgramStep, Register, RegisterList,
    StepBody, StepMemo, StepOperand, StepOperands, StepProgram, StepRows,
};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// The pack a row walks.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// No definition, rule or table.
    Empty,
    /// A definition of a call form (registers 1 and 2) and a table form; a rule of two branches (3; then 4 with an inline body
    /// that sets 5, then 6); and a table of each fill: rows (two pipelines, 10 and 11, then 12), a group (13), a sorted table
    /// (14), an ordered match (15 and 16), a graph of two inputs (17 and 18) and written rows.
    Whole,
}

type Input = Scenario;
/// The registers written by what the walk met, in the order met.
type Value = Cow<'static, [u32]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_pack_has_nothing_to_meet"),
        word: Word::Succeeds,
        input: || Scenario::Empty,
        expect: Expect::Value(Cow::Borrowed(&[])),
    },
    Case {
        name: CaseName("a_packs_walk_meets_every_program_and_every_pipeline_of_every_fill_once"),
        word: Word::Succeeds,
        input: || Scenario::Whole,
        expect: Expect::Value(Cow::Borrowed(&[
            1, 2, 3, 4, 5, 6, 10, 11, 12, 13, 14, 15, 16, 17, 18,
        ])),
    },
];

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

fn whole() -> LoweredPack {
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
                GraphFill::Cycles,
                Box::new([input(17), input(18)]),
            )),
            table(MadeFill::Constant(ConstantRows::default())),
        ],
        ..LoweredPack::default()
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let pack = match scenario {
        Scenario::Empty => LoweredPack::default(),
        Scenario::Whole => whole(),
    };
    let mut writes = Vec::new();
    each_touched(&pack, &mut |touched| {
        writes.extend(touched.writes.iter().map(|register| register.0));
    });
    Ok(Cow::Owned(writes))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
