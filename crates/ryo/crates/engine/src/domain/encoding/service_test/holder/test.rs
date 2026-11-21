//! What the encoding holds, as its backstop: a register keeps its thirty-two bits, a count past its width is the refusal of the
//! rule being written, and a file-sized word stands only first in a rule's branch, first in a pipeline as a `Scan`, or as a
//! `Product`. The lowering proves a pack before it answers it; these are the cases it must never write.

use super::super::fixtures::{listing, mask, of, operands, pack, program, reg, regs, rule, stages};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{Op, ShapeChoice};
use crate::domain::encoding::constants::LAST_STEP;
use crate::domain::encoding::service::encoded;
use crate::domain::lowered::types::{
    DefinitionPattern, Emptiness, LoweredPack, MadeFill, MadeTable, PackDefinition, PatternForm,
};
use crate::domain::lowering::errors::LoweringError;
use crate::domain::lowering::types::{LoadRefusal, PastWidth};
use crate::domain::steps::stages::types::{FillStage, Pipeline, UseMode};
use crate::domain::steps::types::{
    AnchorKinds, DefinitionId, MadeColumn, MadeTableId, MatchColumns, Polarity, ProgramStep,
    ReadColumn, Register, RegisterList, StepBody, StepMemo, StepOperand, StepProgram, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{KindBits, Relation};
use ryo_values::domain::plan_parts::types::RepeatedRows;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// The pack a row encodes.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// A step that names registers past sixteen bits.
    WideRegisters,
    /// An anchor first in a rule's branch, and nothing file-sized after it.
    AnchorFirst,
    /// A pipeline of a scan and a product.
    ProductStage,
    /// A second anchor after the first of a branch.
    AnchorSecond,
    /// An anchor first in an inline body.
    AnchorInBody,
    /// An anchor first in a slot of a `Fork`.
    AnchorInSlot,
    /// An anchor first in a pattern in call form.
    AnchorInPattern,
    /// A use that collects rows with nothing bound, under a candidate.
    UnboundUse,
    /// A scan that is not the first stage of its pipeline.
    ScanSecond,
    /// A probe whose rows go on with no register, first in its pipeline.
    UnkeyedProbeFirst,
    /// A use whose rows go on with nothing bound, in a pipeline.
    UnboundStageUse,
    /// A row of 65,536 registers, in the second of two rules.
    WideRow,
    /// A table of 65,536 columns.
    WideTable,
}

type Input = Scenario;
/// What a row reads of the code written.
type Value = Cow<'static, str>;
type Failure = LoweringError;

/// The encoding's own fault of the first rule.
const FAULT: LoweringError = LoweringError::Refused {
    rule: SelectedRuleOrdinal(0),
    refusal: LoadRefusal::Unlowerable(LAST_STEP),
};

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("register_65_536_is_not_register_0"),
        word: Word::Succeeds,
        input: || Scenario::WideRegisters,
        expect: Expect::Value(Cow::Borrowed(
            "0 End no operand @-\n1 Let LetWord { flag: WordFlag(0), from: WordSlot(65536), out: Register(4000000000) } @0\n2 End no operand @-\n",
        )),
    },
    Case {
        name: CaseName("a_file_sized_step_first_in_a_rules_branch_is_written"),
        word: Word::Succeeds,
        input: || Scenario::AnchorFirst,
        expect: Expect::Value(Cow::Borrowed(
            "0 End no operand @-\n1 End no operand @-\n2 EachKind EachKindWord { flag: WordFlag(0), kind: KindBits(2), out: Register(0), kinds: PoolAt(0) } @0\n3 Exists ExistsWord { flag: WordFlag(0), keys: Short(0), body: PoolAt(0), key: PoolAt(2), memo: MemoSite(4294967295) } @1\n4 End no operand @-\n",
        )),
    },
    Case {
        name: CaseName("a_scan_first_and_a_product_stage_are_written"),
        word: Word::Succeeds,
        input: || Scenario::ProductStage,
        expect: Expect::Value(Cow::Borrowed("1 scans, 1 products, 1 writes")),
    },
    Case {
        name: CaseName("a_file_sized_word_that_is_not_first_and_not_a_product_stage_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AnchorSecond,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("only_a_rule_branchs_entry_not_an_inline_bodys"),
        word: Word::Refuses,
        input: || Scenario::AnchorInBody,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("only_a_rule_branchs_entry_not_a_fork_slots"),
        word: Word::Refuses,
        input: || Scenario::AnchorInSlot,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("only_a_rule_branchs_entry_not_a_call_form_patterns"),
        word: Word::Refuses,
        input: || Scenario::AnchorInPattern,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_use_that_collects_rows_with_nothing_bound_is_refused_under_a_candidate"),
        word: Word::Refuses,
        input: || Scenario::UnboundUse,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_scan_that_is_not_first_in_its_pipeline_is_refused"),
        word: Word::Refuses,
        input: || Scenario::ScanSecond,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_probe_with_no_register_is_refused_even_first_in_its_pipeline"),
        word: Word::Refuses,
        input: || Scenario::UnkeyedProbeFirst,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_stage_use_whose_rows_go_on_with_nothing_bound_is_refused"),
        word: Word::Refuses,
        input: || Scenario::UnboundStageUse,
        expect: Expect::Error(FAULT),
    },
    Case {
        name: CaseName("a_row_of_65_536_registers_is_the_budget_of_the_rule_being_written"),
        word: Word::Refuses,
        input: || Scenario::WideRow,
        expect: Expect::Error(LoweringError::Refused {
            rule: SelectedRuleOrdinal(1),
            refusal: LoadRefusal::Budget(PastWidth::WordOperands),
        }),
    },
    Case {
        name: CaseName("a_table_of_65_536_columns_is_the_budget_of_the_rule_that_reads_it"),
        word: Word::Refuses,
        input: || Scenario::WideTable,
        expect: Expect::Error(LoweringError::Refused {
            rule: SelectedRuleOrdinal(0),
            refusal: LoadRefusal::Budget(PastWidth::TableColumns),
        }),
    },
];

/// An anchor: each id of one kind.
fn anchor() -> ProgramStep {
    ProgramStep::EachKind {
        relation: Relation::SyntaxTree,
        kinds: AnchorKinds(Box::new([KindBits(2)])),
        out: reg(0),
    }
}

/// A scan of the node table into a batch column.
fn scan(out: u32) -> FillStage {
    FillStage::Scan {
        source: StepRows::Node(Relation::SyntaxTree),
        columns: MatchColumns::default(),
        values: operands([]),
        out: reg(out),
    }
}

/// A pack of one rule that walks table 0, whose fill is one pipeline of `filled` and whose rows have `width` columns; with
/// `patterns` of a definition of two columns when a stage uses one.
fn table_pack(
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

/// A pattern of a definition of two columns that binds `bound` and collects the second, in `form`.
fn pattern<const N: usize>(bound: [u16; N], form: PatternForm) -> DefinitionPattern {
    DefinitionPattern {
        bound: mask(bound),
        collected: mask([1]),
        form,
        shape: ShapeChoice::Decided,
    }
}

/// A use of definition 0 that binds nothing and collects its second column.
fn unbound_use() -> StepBody {
    StepBody::Definition {
        definition: DefinitionId(0),
        bound: mask([]),
        collected: mask([1]),
        values: operands([]),
    }
}

/// The pack a row names.
fn lowered(scenario: Scenario) -> LoweredPack {
    let exists = |body| ProgramStep::Exists {
        polarity: Polarity::Present,
        body,
        memo: StepMemo::None,
    };
    match scenario {
        Scenario::WideRegisters => pack(vec![rule(
            0,
            vec![ProgramStep::Let {
                from: of(65_536),
                out: Register(4_000_000_000),
            }],
        )]),
        Scenario::AnchorFirst => pack(vec![rule(
            0,
            vec![anchor(), exists(StepBody::Inline(program(Vec::new())))],
        )]),
        Scenario::ProductStage => table_pack(
            vec![
                scan(1),
                FillStage::Product {
                    source: StepRows::Node(Relation::SyntaxTree),
                    out: reg(2),
                },
            ],
            1,
            Vec::new(),
        ),
        Scenario::AnchorSecond => pack(vec![rule(0, vec![anchor(), anchor()])]),
        Scenario::AnchorInBody => pack(vec![rule(
            0,
            vec![anchor(), exists(StepBody::Inline(program(vec![anchor()])))],
        )]),
        Scenario::AnchorInSlot => pack(vec![rule(
            0,
            vec![
                anchor(),
                ProgramStep::Fork {
                    slots: Box::new([program(vec![anchor()])]),
                },
            ],
        )]),
        Scenario::AnchorInPattern => {
            let keyed = StepBody::Definition {
                definition: DefinitionId(0),
                bound: mask([0]),
                collected: mask([1]),
                values: operands([of(0)]),
            };
            let mut called = pack(vec![rule(0, vec![anchor(), exists(keyed)])]);
            called.definitions = vec![PackDefinition {
                node: NodeDigest([8; 16]),
                formals: regs([40, 41]),
                emptiness: Emptiness::Unknown,
                patterns: vec![pattern([0], PatternForm::Call(program(vec![anchor()])))],
            }];
            called
        }
        Scenario::UnboundUse => {
            let or = ProgramStep::Or {
                bodies: Box::new([unbound_use()]),
                outputs: regs([5]),
                memo: StepMemo::None,
            };
            let mut used = table_pack(
                vec![scan(1)],
                1,
                vec![pattern([], PatternForm::Table(MadeTableId(0)))],
            );
            used.rules = vec![rule(0, vec![anchor(), or])];
            used
        }
        Scenario::ScanSecond => table_pack(vec![scan(1), scan(2)], 1, Vec::new()),
        Scenario::UnkeyedProbeFirst => table_pack(
            vec![FillStage::Probe {
                source: StepRows::Made(MadeTableId(0)),
                columns: MatchColumns(Box::new([ReadColumn::Made(MadeColumn(0))])),
                values: operands([StepOperand::Word(CellWord(3))]),
                range: None,
                mode: UseMode::Rows,
                out: reg(1),
            }],
            1,
            Vec::new(),
        ),
        Scenario::UnboundStageUse => table_pack(
            vec![
                scan(1),
                FillStage::Use {
                    definition: DefinitionId(0),
                    bound: mask([]),
                    collected: mask([1]),
                    values: operands([]),
                    mode: UseMode::Rows,
                    outputs: regs([2]),
                },
            ],
            1,
            vec![pattern([], PatternForm::Table(MadeTableId(0)))],
        ),
        Scenario::WideRow => {
            let row = RegisterList((0..=u32::from(u16::MAX)).map(Register).collect());
            let body = StepBody::Inline(StepProgram(program(vec![ProgramStep::Yield { row }]).0));
            pack(vec![
                rule(0, vec![anchor()]),
                rule(1, vec![anchor(), exists(body)]),
            ])
        }
        Scenario::WideTable => table_pack(vec![scan(1)], usize::from(u16::MAX) + 1, Vec::new()),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let code = encoded(&lowered(scenario))?;
    Ok(Cow::Owned(match scenario {
        Scenario::WideRegisters | Scenario::AnchorFirst => listing(&code),
        Scenario::ProductStage
        | Scenario::AnchorSecond
        | Scenario::AnchorInBody
        | Scenario::AnchorInSlot
        | Scenario::AnchorInPattern
        | Scenario::UnboundUse
        | Scenario::ScanSecond
        | Scenario::UnkeyedProbeFirst
        | Scenario::UnboundStageUse
        | Scenario::WideRow
        | Scenario::WideTable => {
            let stage = |op: Op| code.words.iter().filter(|word| word.op == op).count();
            format!(
                "{} scans, {} products, {} writes",
                stage(Op::StageScan),
                stage(Op::StageProduct),
                stage(Op::StageWrite)
            )
        }
    }))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
