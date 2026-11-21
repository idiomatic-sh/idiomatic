//! The cases of `dispatch` over fills, the budget's charge and the scan's counters, over a fixture pack the fill side
//! makes: a table of `n` constants
//! (table 0), and a table (table 1) whose one pipeline pairs every constant with every constant (a product stage), keeps
//! the pairs whose second is nought, and asks a pattern in call form for each row left. A rule reads one of the tables
//! and emits each row.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::{HeaviestWork, RuleColumns};
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::made_tables::fixtures::packs::{
    constants, definition, keep_equal, of, operands, pipeline, rows, scan_made, used,
};
use crate::domain::programs::fixtures::{framed, held};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::stages::types::{FillStage, UseMode};
use crate::domain::steps::types::{
    EmitTarget, MadeColumn, MadeTableId, PlacedStep, ProgramStep, ReadColumn, Register,
    RegisterCount, StepProgram, StepRows,
};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// How many constants table 0 holds, and the table each rule reads, in rule order.
type Input = (u64, &'static [u32]);
/// For each notice the rule, the work, the allowance, the heaviest unit's kind (0 the rule's own program, 1 a called
/// pattern, 2 a product, 3 a fill of rows) and its two counts; then 99, how many rows were found, and the rows the scan
/// counted as made and as read and given by fills.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_rule_over_a_table_whose_fill_pairs_every_row_is_noticed_with_the_product",
        ),
        word: Word::Succeeds,
        input: || (150, &[1]),
        expect: Expect::Value(Cow::Borrowed(&[
            0, 91500, 57600, 2, 150, 150, 99, 150, 300, 91350,
        ])),
    },
    Case {
        name: CaseName("the_same_rule_over_fewer_rows_is_within_its_allowance"),
        word: Word::Succeeds,
        input: || (40, &[1]),
        expect: Expect::Value(Cow::Borrowed(&[99, 40, 80, 6760])),
    },
    Case {
        name: CaseName("a_second_rule_over_the_table_is_charged_it_whole_as_the_rule_that_made_it"),
        word: Word::Succeeds,
        input: || (150, &[1, 1]),
        expect: Expect::Value(Cow::Borrowed(&[
            0, 91500, 57600, 2, 150, 150, 1, 91500, 57600, 2, 150, 150, 99, 300, 300, 91350,
        ])),
    },
    Case {
        name: CaseName(
            "a_table_its_fill_read_is_charged_through_the_list_when_another_rule_made_it",
        ),
        word: Word::Succeeds,
        input: || (150, &[0, 1]),
        expect: Expect::Value(Cow::Borrowed(&[
            1, 91500, 57600, 2, 150, 150, 99, 300, 300, 91350,
        ])),
    },
];

/// The rule set of a case.
fn pack((count, read): Input) -> LoweredPack {
    let cells: Vec<u64> = (0..count).collect();
    let mut stages = Vec::from(scan_made(0, 0, 1));
    stages.extend([
        FillStage::Product {
            source: StepRows::Made(MadeTableId(0)),
            out: Register(3),
        },
        FillStage::Read {
            source: StepRows::Made(MadeTableId(0)),
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(3),
            out: Register(4),
        },
        keep_equal(4, 0),
        used(1, UseMode::Rows, &[5]),
    ]);
    let rules = read
        .iter()
        .enumerate()
        .map(|(at, table)| {
            let number = u32::try_from(at).unwrap_or(u32::MAX);
            let steps = [
                ProgramStep::EachRow {
                    source: StepRows::Made(MadeTableId(*table)),
                    out: Register(900_u32.saturating_add(number)),
                },
                ProgramStep::Emit {
                    target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
                    columns: operands([]),
                },
            ];
            LoweredRule {
                rule: SelectedRuleOrdinal(number),
                node: NodeDigest([u8::try_from(at).unwrap_or(u8::MAX); 16]),
                branches: vec![Branch {
                    emptiness: Emptiness::Unknown,
                    program: StepProgram(
                        steps
                            .into_iter()
                            .map(|step| PlacedStep { point: None, step })
                            .collect(),
                    ),
                }],
                columns: RuleColumns::default(),
            }
        })
        .collect();
    LoweredPack {
        definitions: vec![definition()],
        tables: vec![constants(&cells), rows(2, pipeline(stages, [of(1), of(5)]))],
        rules,
        registers: RegisterCount(1000),
        ..LoweredPack::default()
    }
}

fn subject(input: Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(input)).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let mut held = held(1000);
    let (counted, found) = framed(&programs.0, &mut held, &TheLoop, |frame| {
        dispatch(frame, &programs)
    });
    let mut all = Vec::new();
    for notice in &found.notices {
        let (kind, one, other) = match notice.heaviest {
            HeaviestWork::Program {
                candidates,
                visited,
            } => (0, candidates, visited),
            HeaviestWork::Calls { keys, visited } => (1, keys, visited),
            HeaviestWork::Product { left, right } => (2, left, right),
            HeaviestWork::Rows { read, made } => (3, read, made),
            HeaviestWork::Group { .. }
            | HeaviestWork::Sorted { .. }
            | HeaviestWork::OrderedPairs { .. }
            | HeaviestWork::Graph { .. } => return Err(()),
        };
        all.extend([
            u64::from(notice.rule.0),
            notice.work.0,
            notice.allowance.0,
            kind,
            one.0,
            other.0,
        ]);
    }
    all.extend([
        99,
        u64::try_from(found.rows.len()).unwrap_or(u64::MAX),
        counted.made_rows.0,
        counted.fill_rows.0,
    ]);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
