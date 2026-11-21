//! The cases of `dispatch` over two asks the loop answers cheaply. One rule reads three constants and, for each, asks
//! either a pattern in call form for the constant key 7 (one answer a file: evaluated once, then the site's slot), or an
//! inline body of one `Yield` (it has a row: no run is entered), and emits the row. One worker scans the files.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::made_tables::fixtures::packs::{constants, definition, operands, word};
use crate::domain::programs::fixtures::{Held, framed};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::scratch::service::next_file;
use crate::domain::steps::types::{
    ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId, PlacedStep, Polarity,
    ProgramStep, Register, RegisterCount, RegisterList, StepBody, StepMemo, StepProgram, StepRows,
};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// What the rule asks for each row.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// The pattern in call form, for the constant key 7.
    ConstantKey,
    /// An inline body of one `Yield`.
    OneYield,
}

/// What is asked, and how many files the one worker scans.
type Input = (Asked, usize);
/// For each file: the rows found, the steps the scan counted, and the keys of call patterns charged to the rule.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_pattern_asked_with_a_constant_key_is_evaluated_once_a_file"),
        word: Word::Succeeds,
        input: || (Asked::ConstantKey, 1),
        expect: Expect::Value(Cow::Borrowed(&[3, 11, 1])),
    },
    Case {
        name: CaseName("the_next_file_evaluates_the_constant_key_again"),
        word: Word::Succeeds,
        input: || (Asked::ConstantKey, 2),
        expect: Expect::Value(Cow::Borrowed(&[3, 11, 1, 3, 11, 1])),
    },
    Case {
        name: CaseName("a_body_of_one_yield_has_a_row_and_no_run_is_entered"),
        word: Word::Succeeds,
        input: || (Asked::OneYield, 1),
        expect: Expect::Value(Cow::Borrowed(&[3, 10, 0])),
    },
];

/// The rule set of a case.
fn pack(asked: Asked) -> LoweredPack {
    let body = match asked {
        Asked::ConstantKey => StepBody::Definition {
            definition: DefinitionId(0),
            bound: ColumnMask(Box::new([MadeColumn(0)])),
            collected: ColumnMask(Box::new([])),
            values: operands([word(7)]),
        },
        Asked::OneYield => StepBody::Inline(StepProgram(Box::new([PlacedStep {
            point: None,
            step: ProgramStep::Yield {
                row: RegisterList(Box::new([])),
            },
        }]))),
    };
    let steps = [
        ProgramStep::EachRow {
            source: StepRows::Made(MadeTableId(0)),
            out: Register(0),
        },
        ProgramStep::Exists {
            polarity: Polarity::Present,
            body,
            memo: StepMemo::None,
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: operands([]),
        },
    ];
    LoweredPack {
        definitions: vec![definition()],
        tables: vec![constants(&[1, 2, 3])],
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
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
        }],
        registers: RegisterCount(1000),
        ..LoweredPack::default()
    }
}

fn subject((asked, files): Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(asked)).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let mut worker = scratch_for(&programs);
    let mut said = Vec::new();
    for _ in 0..files {
        next_file(&mut worker);
        // The loop's fixtures frame a state and a store: the worker's are lent to them for the file.
        let mut held = Held {
            state: std::mem::take(&mut worker.0.state),
            store: std::mem::take(&mut worker.0.store),
        };
        let (counted, found) = framed(&programs.0, &mut held, &TheLoop, |frame| {
            dispatch(frame, &programs)
        });
        said.extend([
            u64::try_from(found.rows.len()).unwrap_or(u64::MAX),
            counted.steps.0,
            held.state.budget.keys.0,
        ]);
        worker.0.state = held.state;
        worker.0.store = held.store;
    }
    Ok(Cow::Owned(said))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
