//! The cases of `dispatch` over two asks the loop answers cheaply. One rule reads three constants and, for each, asks
//! either a pattern in call form for the constant key 7 (one answer a file: evaluated once, then the site's slot), or an
//! inline body of one `Yield` (it has a row: no run is entered), or a pattern in call form whose definition cannot have
//! a row in the file (it is not asked: no key, no run, no answer kept), and emits the row. One worker scans the files.
//!
//! SPIKE (2026-11-21): the fixture's pattern is one `Yield`, a short straight line, so since the quick asks it is run in
//! place at every ask and no key is charged; the cases' names that say "evaluated once" and "constant key" predate that
//! and want a pattern past the bound (`code::constants::STRAIGHT_WORDS`) when the spike is hardened.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::ShapeChoice;
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, DefinitionPattern, Emptiness, LoweredPack, LoweredRule, PackDefinition, PatternForm,
};
use crate::domain::made_tables::fixtures::packs::{constants, definition, of, operands, word};
use crate::domain::programs::fixtures::{Held, framed_over};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::scratch::service::next_file;
use crate::domain::steps::types::{
    AnchorKinds, ColumnMask, DefinitionId, EmitTarget, MadeColumn, MadeTableId, PlacedStep,
    Polarity, ProgramStep, Register, RegisterCount, RegisterList, StepBody, StepMemo, StepProgram,
    StepRows,
};
use ryo_language::domain::node_table::service::{builder, enter_node, finished, leave_node};
use ryo_language::domain::node_table::types::{FieldCode, NodeTable};
use ryo_relations::domain::relations::types::{Count, KindBits, Offset};
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
    /// The pattern in call form, for the row's own key, kept when it has NO row; its definition's need is a node of
    /// kind 2, which the second file alone holds.
    NoRowOfAKind,
    /// The same, kept when it HAS a row.
    RowOfAKind,
    /// The pattern in call form, for the row's own key, kept when it has no row; its definition's need says nothing.
    NoRowOfUnknown,
    /// A pattern in call form whose program is its end alone (it has no row), for the row's own key, kept when it HAS a
    /// row.
    RowOfAnEmptyProgram,
    /// The same pattern, kept when it has NO row.
    NoRowOfAnEmptyProgram,
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
        expect: Expect::Value(Cow::Borrowed(&[3, 13, 0])),
    },
    Case {
        name: CaseName("the_next_file_evaluates_the_constant_key_again"),
        word: Word::Succeeds,
        input: || (Asked::ConstantKey, 2),
        expect: Expect::Value(Cow::Borrowed(&[3, 13, 0, 3, 13, 0])),
    },
    Case {
        name: CaseName("a_body_of_one_yield_has_a_row_and_no_run_is_entered"),
        word: Word::Succeeds,
        input: || (Asked::OneYield, 1),
        expect: Expect::Value(Cow::Borrowed(&[3, 10, 0])),
    },
    Case {
        name: CaseName("a_pattern_whose_definition_cannot_have_a_row_in_the_file_is_not_asked"),
        word: Word::Succeeds,
        input: || (Asked::NoRowOfAKind, 1),
        expect: Expect::Value(Cow::Borrowed(&[3, 10, 0])),
    },
    Case {
        name: CaseName(
            "it_is_asked_in_the_file_where_its_definition_may_have_a_row_and_not_in_the_next",
        ),
        word: Word::Succeeds,
        input: || (Asked::NoRowOfAKind, 3),
        expect: Expect::Value(Cow::Borrowed(&[3, 10, 0, 0, 7, 0, 3, 10, 0])),
    },
    Case {
        name: CaseName("a_row_wanted_of_a_definition_that_cannot_have_one_ends_each_candidate"),
        word: Word::Succeeds,
        input: || (Asked::RowOfAKind, 2),
        expect: Expect::Value(Cow::Borrowed(&[0, 4, 0, 3, 13, 0])),
    },
    Case {
        name: CaseName("a_pattern_whose_definitions_need_says_nothing_is_asked_for_every_key"),
        word: Word::Succeeds,
        input: || (Asked::NoRowOfUnknown, 1),
        expect: Expect::Value(Cow::Borrowed(&[0, 7, 0])),
    },
    Case {
        name: CaseName(
            "a_pattern_whose_program_is_its_end_alone_has_no_row_for_an_exists_that_wants_one",
        ),
        word: Word::Succeeds,
        input: || (Asked::RowOfAnEmptyProgram, 1),
        expect: Expect::Value(Cow::Borrowed(&[0, 7, 0])),
    },
    Case {
        name: CaseName(
            "a_pattern_whose_program_is_its_end_alone_has_no_row_for_an_exists_that_wants_none",
        ),
        word: Word::Succeeds,
        input: || (Asked::NoRowOfAnEmptyProgram, 1),
        expect: Expect::Value(Cow::Borrowed(&[3, 13, 0])),
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
        Asked::NoRowOfAKind
        | Asked::RowOfAKind
        | Asked::NoRowOfUnknown
        | Asked::RowOfAnEmptyProgram
        | Asked::NoRowOfAnEmptyProgram => StepBody::Definition {
            definition: DefinitionId(0),
            bound: ColumnMask(Box::new([MadeColumn(0)])),
            collected: ColumnMask(Box::new([])),
            values: operands([of(0)]),
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
            polarity: match asked {
                Asked::NoRowOfAKind | Asked::NoRowOfUnknown | Asked::NoRowOfAnEmptyProgram => {
                    Polarity::Absent
                }
                Asked::ConstantKey
                | Asked::OneYield
                | Asked::RowOfAKind
                | Asked::RowOfAnEmptyProgram => Polarity::Present,
            },
            body,
            memo: StepMemo::None,
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: operands([]),
        },
    ];
    LoweredPack {
        definitions: vec![PackDefinition {
            emptiness: match asked {
                Asked::NoRowOfAKind | Asked::RowOfAKind => {
                    Emptiness::Kind(AnchorKinds(Box::new([KindBits(2)])))
                }
                Asked::ConstantKey
                | Asked::OneYield
                | Asked::NoRowOfUnknown
                | Asked::RowOfAnEmptyProgram
                | Asked::NoRowOfAnEmptyProgram => Emptiness::Unknown,
            },
            patterns: match asked {
                // A pattern whose program holds no step: its code is its end alone.
                Asked::RowOfAnEmptyProgram | Asked::NoRowOfAnEmptyProgram => {
                    vec![DefinitionPattern {
                        bound: ColumnMask(Box::new([MadeColumn(0)])),
                        collected: ColumnMask(Box::new([])),
                        form: PatternForm::Call(StepProgram(Box::new([]))),
                        shape: ShapeChoice::Decided,
                    }]
                }
                Asked::ConstantKey
                | Asked::OneYield
                | Asked::NoRowOfAKind
                | Asked::RowOfAKind
                | Asked::NoRowOfUnknown => definition().patterns,
            },
            ..definition()
        }],
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
    for file in 0..files {
        // The second file alone has nodes: a root of kind 1 with a child of kind 2.
        let mut nodes = NodeTable::default();
        if file == 1 {
            let mut writer = builder(&mut nodes, Count(4));
            enter_node(&mut writer, KindBits(1), FieldCode(0), Offset(0));
            enter_node(&mut writer, KindBits(2), FieldCode(0), Offset(0));
            leave_node(&mut writer, Offset(2));
            leave_node(&mut writer, Offset(8));
            let _table = finished(&mut writer);
        }
        next_file(&mut worker);
        // The loop's fixtures frame a state and a store: the worker's are lent to them for the file.
        let mut held = Held {
            state: std::mem::take(&mut worker.0.state),
            store: std::mem::take(&mut worker.0.store),
        };
        let (counted, found) = framed_over(&nodes, &programs.0, &mut held, &TheLoop, |frame| {
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
