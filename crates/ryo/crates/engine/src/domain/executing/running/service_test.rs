//! The cases of the walk that asks whether the executor runs all a rule can reach, over fixture packs of two rules written
//! as steps and encoded by the straight encoding. The second rule always reads a table of constants and emits: it runs.

use super::service::unit_not_run;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{ShapeChoice, UnitKey};
use crate::domain::encoding::service::encoded;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, PackDefinition, PatternForm,
};
use crate::domain::made_tables::fixtures::packs::{
    constants, of, operands, pipeline, rows, scan_made,
};
use crate::domain::steps::stages::types::FillStage;
use crate::domain::steps::types::{
    CallMeaning, ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId, PlacedStep,
    Polarity, ProgramStep, ReadColumn, Register, RegisterCount, RegisterList, StepBody, StepMemo,
    StepOperand, StepOperands, StepProgram, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// What the first rule holds between its anchor over the constants and its emit.
#[derive(Clone, Copy, Debug)]
enum Scene {
    /// Nothing: a read of the row.
    AllRun,
    /// A word the loop does not run: a call of a per-row meaning.
    CallInTheRule,
    /// A test of a text the executor does not run.
    PrefixTest,
    /// An `Exists` over a pattern in call form whose program holds a call.
    CallInAPattern,
    /// A read of a second table whose fill's pipeline keeps by a test of a text the fill side does not run.
    PrefixKeepInAFill,
}

type Input = Scene;
/// For each of the two rules, what the unit it cannot run is: 9 none, 0 the rule's own program, 1 a pattern, 2 a table's
/// fill.
type Value = [u8; 2];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_of_words_the_executor_runs_is_run_whole"),
        word: Word::Succeeds,
        input: || Scene::AllRun,
        expect: Expect::Value([9, 9]),
    },
    Case {
        name: CaseName("a_word_the_loop_does_not_run_names_the_rules_own_unit"),
        word: Word::Succeeds,
        input: || Scene::CallInTheRule,
        expect: Expect::Value([0, 9]),
    },
    Case {
        name: CaseName("a_test_of_a_text_the_executor_does_not_run_names_the_rules_own_unit"),
        word: Word::Succeeds,
        input: || Scene::PrefixTest,
        expect: Expect::Value([0, 9]),
    },
    Case {
        name: CaseName(
            "a_word_in_a_pattern_the_rule_asks_names_the_patterns_unit_for_that_rule_alone",
        ),
        word: Word::Succeeds,
        input: || Scene::CallInAPattern,
        expect: Expect::Value([1, 9]),
    },
    Case {
        name: CaseName("a_stage_in_the_fill_of_a_table_the_rule_reads_names_the_tables_unit"),
        word: Word::Succeeds,
        input: || Scene::PrefixKeepInAFill,
        expect: Expect::Value([2, 9]),
    },
];

/// A call of a per-row meaning: a word the loop does not run.
fn call() -> ProgramStep {
    ProgramStep::Call {
        meaning: CallMeaning::Lower,
        arguments: StepOperands(Box::new([])),
        outputs: RegisterList(Box::new([])),
    }
}

/// A program of steps with no plan point.
fn program(steps: Vec<ProgramStep>) -> StepProgram {
    StepProgram(
        steps
            .into_iter()
            .map(|step| PlacedStep { point: None, step })
            .collect(),
    )
}

/// A rule over a table: its anchor, a read of the row's cell, `between`, an emit.
fn rule(number: u32, table: u32, between: Vec<ProgramStep>) -> LoweredRule {
    let made = StepRows::Made(MadeTableId(table));
    let mut steps = vec![
        ProgramStep::EachRow {
            source: made,
            out: Register(0),
        },
        ProgramStep::Read {
            source: made,
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(0),
            out: Register(1),
        },
    ];
    steps.extend(between);
    steps.push(ProgramStep::Emit {
        target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
        columns: operands([]),
    });
    LoweredRule {
        rule: SelectedRuleOrdinal(number),
        node: NodeDigest([u8::try_from(number).unwrap_or(u8::MAX); 16]),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: program(steps),
        }],
        columns: RuleColumns::default(),
    }
}

/// The rule set of a scene.
fn pack(scene: Scene) -> LoweredPack {
    let prefix = |left: StepOperand| {
        (
            TestOperation::StartsWith,
            left,
            StepOperand::Word(CellWord(0)),
        )
    };
    let (first, between) = match scene {
        Scene::AllRun => (0, Vec::new()),
        Scene::CallInTheRule => (0, vec![call()]),
        Scene::PrefixTest => {
            let (test, left, right) = prefix(of(1));
            (0, vec![ProgramStep::Test { test, left, right }])
        }
        Scene::CallInAPattern => (
            0,
            vec![ProgramStep::Exists {
                polarity: Polarity::Present,
                body: StepBody::Definition {
                    definition: DefinitionId(0),
                    bound: ColumnMask(Box::new([MadeColumn(0)])),
                    collected: ColumnMask(Box::new([])),
                    values: operands([of(1)]),
                },
                memo: StepMemo::None,
            }],
        ),
        Scene::PrefixKeepInAFill => (1, Vec::new()),
    };
    let mut stages = Vec::from(scan_made(0, 10, 11));
    let (test, left, right) = prefix(of(11));
    stages.push(FillStage::Keep { test, left, right });
    LoweredPack {
        definitions: vec![PackDefinition {
            node: NodeDigest([80; 16]),
            formals: RegisterList(Box::new([Register(5)])),
            emptiness: Emptiness::Unknown,
            patterns: vec![DefinitionPattern {
                bound: ColumnMask(Box::new([MadeColumn(0)])),
                collected: ColumnMask(Box::new([])),
                form: PatternForm::Call(program(vec![
                    call(),
                    ProgramStep::Yield {
                        row: RegisterList(Box::new([])),
                    },
                ])),
                shape: ShapeChoice::Decided,
            }],
        }],
        tables: vec![constants(&[1, 2]), rows(1, pipeline(stages, [of(11)]))],
        rules: vec![rule(0, first, between), rule(1, 0, Vec::new())],
        registers: RegisterCount(20),
        ..LoweredPack::default()
    }
}

fn subject(scene: Input) -> Result<Value, Failure> {
    let code = encoded(&pack(scene)).map_err(|_| ())?;
    let said = |number: usize| {
        let unit = code
            .rules
            .get(number)
            .and_then(|rule| unit_not_run(&code, rule));
        let key = unit.and_then(|unit| {
            code.units
                .get(usize::try_from(unit.0).unwrap_or(usize::MAX))
        });
        match key.map(|unit| &unit.key) {
            None => 9,
            Some(UnitKey::Rule(_)) => 0,
            Some(UnitKey::Pattern(..)) => 1,
            Some(UnitKey::Table(..)) => 2,
        }
    };
    Ok([said(0), said(1)])
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
