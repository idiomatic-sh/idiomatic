//! The cases of the `proving` part: packs built by hand, each one step or stage away from the invariant. A file-sized item
//! stands only first in a rule's branch, first in a pipeline as its scan, or as a product stage of a table whose plan states
//! a keyless join; a definition's rows are read whole only there.

use super::service::proved;
use super::types::{ProductStated, ProofFault, ProvedUnit, Unproved};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable,
    PackDefinition, PatternForm,
};
use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::stages::types::{FillStage, Pipeline, PlacedStage, UseMode};
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId, MatchColumns, PlacedStep,
    ProgramStep, ReadColumn, Register, RegisterList, StepBody, StepMemo, StepOperand,
    StepOperands, StepProgram, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{NodeColumn, Relation};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// A pack, and whether the plan of each of its tables states a keyless join.
type Input = (LoweredPack, Vec<ProductStated>);
type Value = ();
type Failure = Unproved;

fn program(steps: Vec<ProgramStep>) -> StepProgram {
    StepProgram(steps.into_iter().map(|step| PlacedStep { point: None, step }).collect())
}

/// Every element of the node table: file-sized.
fn anchor() -> ProgramStep {
    ProgramStep::EachRow {
        source: StepRows::Node(Relation::SyntaxTree),
        out: Register(0),
    }
}

/// The rows of the node table an element is the parent of: keyed by a register.
fn keyed() -> ProgramStep {
    ProgramStep::EachMatch {
        source: StepRows::Node(Relation::SyntaxChild),
        columns: MatchColumns([ReadColumn::Node(NodeColumn::Parent)].into()),
        values: StepOperands([StepOperand::Register(Register(0))].into()),
        out: Register(1),
    }
}

fn emit() -> ProgramStep {
    ProgramStep::Emit {
        target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
        columns: StepOperands([StepOperand::Register(Register(0))].into()),
    }
}

/// A use of definition 0 that binds `bound` columns to register 0 and collects `collected` columns.
fn used(bound: &[u16], collected: &[u16]) -> StepBody {
    let mask = |columns: &[u16]| ColumnMask(columns.iter().copied().map(MadeColumn).collect());
    StepBody::Definition {
        definition: DefinitionId(0),
        bound: mask(bound),
        collected: mask(collected),
        values: StepOperands(bound.iter().map(|_| StepOperand::Register(Register(0))).collect()),
    }
}

/// A pack of one rule whose one branch is the steps.
fn rule_of(steps: Vec<ProgramStep>) -> Input {
    let rule = LoweredRule {
        rule: SelectedRuleOrdinal(0),
        node: NodeDigest::default(),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: program(steps),
        }],
        columns: RuleColumns::default(),
    };
    let pack = LoweredPack {
        rules: vec![rule],
        ..LoweredPack::default()
    };
    (pack, Vec::new())
}

/// A pack of one definition whose one pattern, in call form, is the steps.
fn pattern_of(steps: Vec<ProgramStep>) -> Input {
    let definition = PackDefinition {
        node: NodeDigest::default(),
        formals: RegisterList([Register(0)].into()),
        emptiness: Emptiness::Unknown,
        patterns: vec![DefinitionPattern {
            bound: ColumnMask([MadeColumn(0)].into()),
            collected: ColumnMask::default(),
            form: PatternForm::Call(program(steps)),
        }],
    };
    let pack = LoweredPack {
        definitions: vec![definition],
        ..LoweredPack::default()
    };
    (pack, Vec::new())
}

fn scan() -> FillStage {
    FillStage::Scan {
        source: StepRows::Filled(Relation::SyntaxError),
        columns: MatchColumns::default(),
        values: StepOperands::default(),
        out: Register(0),
    }
}

/// A probe of a filled relation in rows mode, by the value.
fn probe(value: StepOperand) -> FillStage {
    FillStage::Probe {
        source: StepRows::Filled(Relation::Function),
        columns: MatchColumns::default(),
        values: StepOperands([value].into()),
        range: None,
        mode: UseMode::Rows,
        out: Register(1),
    }
}

fn product() -> FillStage {
    FillStage::Product {
        source: StepRows::Filled(Relation::Function),
        out: Register(1),
    }
}

/// A pack of one table of a definition's rows whose one pipeline is the stages, with whether its plan states a keyless join.
fn table_of(stages: Vec<FillStage>, row: usize, stated: ProductStated) -> Input {
    let pipeline = Pipeline {
        stages: stages.into_iter().map(|stage| PlacedStage { point: None, stage }).collect(),
        row: StepOperands((0..row).map(|_| StepOperand::File).collect()),
    };
    let table = MadeTable {
        node: NodeDigest::default(),
        fill: MadeFill::Rows([pipeline].into(), RepeatedRows::Dropped),
        width: ColumnCount(row),
        indexes: Vec::new(),
        emptiness: Emptiness::Unknown,
    };
    let pack = LoweredPack {
        tables: vec![table],
        ..LoweredPack::default()
    };
    (pack, vec![stated])
}

const fn misplaced(unit: ProvedUnit) -> Expect<Value, Failure> {
    Expect::Error(Unproved {
        unit,
        fault: ProofFault::Misplaced,
    })
}

const TABLE: ProvedUnit = ProvedUnit::Table(MadeTableId(0));
const PATTERN: ProvedUnit = ProvedUnit::Definition(DefinitionId(0));

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_sized_step_stands_first_in_a_rules_branch"),
        word: Word::Succeeds,
        input: || rule_of(vec![anchor(), keyed(), emit()]),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_file_sized_step_after_a_branchs_first_is_refused"),
        word: Word::Refuses,
        input: || rule_of(vec![anchor(), anchor(), emit()]),
        expect: misplaced(ProvedUnit::Rule(0)),
    },
    Case {
        name: CaseName("a_file_sized_step_in_a_call_form_pattern_is_refused"),
        word: Word::Refuses,
        input: || pattern_of(vec![anchor()]),
        expect: misplaced(PATTERN),
    },
    Case {
        name: CaseName("a_file_sized_step_in_a_slot_of_a_fork_is_refused"),
        word: Word::Refuses,
        input: || {
            let slot = ProgramStep::Fork {
                slots: [program(vec![anchor()])].into(),
            };
            rule_of(vec![anchor(), slot, emit()])
        },
        expect: misplaced(ProvedUnit::Rule(0)),
    },
    Case {
        name: CaseName("a_use_that_collects_rows_with_nothing_bound_under_a_candidate_is_refused"),
        word: Word::Refuses,
        input: || {
            let unkeyed = ProgramStep::Or {
                bodies: [used(&[], &[1])].into(),
                outputs: RegisterList([Register(2)].into()),
                memo: StepMemo::None,
            };
            rule_of(vec![anchor(), unkeyed, emit()])
        },
        expect: misplaced(ProvedUnit::Rule(0)),
    },
    Case {
        name: CaseName("a_definitions_rows_read_whole_after_a_branchs_first_step_are_refused"),
        word: Word::Refuses,
        input: || {
            let whole = ProgramStep::EachRow {
                source: StepRows::Definition(DefinitionId(0)),
                out: Register(3),
            };
            rule_of(vec![anchor(), whole, emit()])
        },
        expect: misplaced(ProvedUnit::Rule(0)),
    },
    Case {
        name: CaseName("a_pipeline_of_a_scan_and_a_probe_by_a_register_holds"),
        word: Word::Succeeds,
        input: || table_of(vec![scan(), probe(StepOperand::Register(Register(0)))], 1, ProductStated::Keyed),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_keyed_join_has_no_product_stage"),
        word: Word::Refuses,
        input: || table_of(vec![scan(), product()], 1, ProductStated::Keyed),
        expect: misplaced(TABLE),
    },
    Case {
        name: CaseName("a_product_stage_stands_in_the_table_of_a_keyless_join"),
        word: Word::Succeeds,
        input: || table_of(vec![scan(), product()], 1, ProductStated::Stated),
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("a_probe_on_constants_alone_is_refused"),
        word: Word::Refuses,
        input: || table_of(vec![scan(), probe(StepOperand::Word(CellWord(1)))], 1, ProductStated::Stated),
        expect: misplaced(TABLE),
    },
    Case {
        name: CaseName("a_scan_that_is_not_first_is_refused"),
        word: Word::Refuses,
        input: || table_of(vec![scan(), scan()], 1, ProductStated::Stated),
        expect: misplaced(TABLE),
    },
    Case {
        name: CaseName("a_pipeline_that_does_not_start_with_a_scan_is_refused"),
        word: Word::Refuses,
        input: || table_of(vec![product()], 1, ProductStated::Stated),
        expect: misplaced(TABLE),
    },
    Case {
        name: CaseName("a_row_of_65_536_operands_is_past_a_words_operands"),
        word: Word::Refuses,
        input: || table_of(vec![scan()], 65_536, ProductStated::Keyed),
        expect: Expect::Error(Unproved {
            unit: TABLE,
            fault: ProofFault::Past(PastWidth::WordOperands),
        }),
    },
];

fn subject((pack, products): Input) -> Result<Value, Failure> {
    proved(&pack, &products)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
