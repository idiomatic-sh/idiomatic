//! SPIKE (2026-11-21): rule sets shaped like a pack for the reference bench: many rules, units, memo sites and tables,
//! few candidates a rule.

use super::workloads::LoopBench;
use crate::domain::code::types::ShapeChoice;
use crate::domain::encoding::service::encoded;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, MadeFill,
    MadeTable, PackDefinition, PatternForm,
};
use crate::domain::made_tables::fixtures::packs::{of, operands};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    AnchorKinds, ColumnList, ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId,
    PlacedStep, Polarity, ProgramStep, Register, RegisterCount, RegisterList, StepBody, StepMemo,
    StepOperand, StepProgram, StepRows, TestOperation,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{CellKind, KindBits};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;

/// A program of steps with no plan point.
fn program(steps: Vec<ProgramStep>) -> StepProgram {
    StepProgram(
        steps
            .into_iter()
            .map(|step| PlacedStep { point: None, step })
            .collect(),
    )
}

/// A digest that differs for each number.
fn digest(kind: u8, number: usize) -> NodeDigest {
    let mut bytes = [kind; 16];
    for (place, byte) in number.to_le_bytes().iter().enumerate() {
        if let Some(slot) = bytes.get_mut(place) {
            *slot = *byte;
        }
    }
    NodeDigest(bytes)
}

/// The rule set of a pack-shaped workload of `rules` rules.
pub(super) fn pack_programs(bench: LoopBench, rules: usize) -> Option<Programs> {
    let shared = rules.div_ceil(4).max(1);
    let tables: Vec<MadeTable> = (0..shared)
        .map(|number| MadeTable {
            node: digest(90, number),
            fill: MadeFill::Constant(ConstantRows(vec![
                Box::new([CellWord(1)]),
                Box::new([CellWord(2)]),
            ])),
            width: ColumnCount(1),
            indexes: vec![ColumnList(Box::new([MadeColumn(0)]))],
            emptiness: Emptiness::Unknown,
        })
        .collect();
    // A pattern of five words: past the straight bound, so its answers are kept by key.
    let test = |cell: u64| ProgramStep::Test {
        test: TestOperation::NotEqual,
        left: of(5),
        right: StepOperand::Word(CellWord(cell)),
    };
    let definitions: Vec<PackDefinition> = (0..shared)
        .map(|number| PackDefinition {
            node: digest(80, number),
            formals: RegisterList(Box::new([Register(5)])),
            emptiness: Emptiness::Unknown,
            patterns: vec![DefinitionPattern {
                bound: ColumnMask(Box::new([MadeColumn(0)])),
                collected: ColumnMask(Box::new([])),
                form: PatternForm::Call(program(vec![
                    test(7),
                    test(8),
                    test(9),
                    test(10),
                    ProgramStep::Yield {
                        row: RegisterList(Box::new([])),
                    },
                ])),
                shape: ShapeChoice::Decided,
            }],
        })
        .collect();
    let rule = |number: usize| -> Option<LoweredRule> {
        let ordinal = SelectedRuleOrdinal(u32::try_from(number).ok()?);
        let shared_number = u32::try_from(number / 4).ok()?;
        let steps = vec![
            ProgramStep::EachRow {
                source: StepRows::Made(MadeTableId(shared_number)),
                out: Register(0),
            },
            ProgramStep::Exists {
                polarity: Polarity::Present,
                body: StepBody::Definition {
                    definition: DefinitionId(shared_number),
                    bound: ColumnMask(Box::new([MadeColumn(0)])),
                    collected: ColumnMask(Box::new([])),
                    values: operands([of(0)]),
                },
                memo: StepMemo::None,
            },
            ProgramStep::Emit {
                target: EmitTarget::Rule(ordinal),
                columns: operands([of(0)]),
            },
        ];
        let idle = bench == LoopBench::PackIdle && number > 0;
        Some(LoweredRule {
            rule: ordinal,
            node: digest(1, number),
            branches: vec![Branch {
                emptiness: if idle {
                    Emptiness::Kind(AnchorKinds(Box::new([KindBits(2)])))
                } else {
                    Emptiness::Unknown
                },
                program: program(steps),
            }],
            columns: RuleColumns {
                emit: ColumnKinds(vec![CellKind::Number]),
                frontier: Vec::new(),
            },
        })
    };
    let pack = LoweredPack {
        definitions,
        tables,
        rules: (0..rules).map(rule).collect::<Option<Vec<_>>>()?,
        registers: RegisterCount(8),
        ..LoweredPack::default()
    };
    Some(Programs(ProgramsEngine {
        code: encoded(&pack).ok()?,
        ..ProgramsEngine::default()
    }))
}
