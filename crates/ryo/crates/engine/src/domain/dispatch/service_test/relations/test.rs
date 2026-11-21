//! The cases of `dispatch` over a need on a filled relation's row. One rule reads every row of a relation the language
//! fills with five rows, each cell its row's number, and emits it; its need says the relation holds a row with a given
//! cell in its second column, or any row. A rule whose need does not hold is not run.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule, RowValue};
use crate::domain::made_tables::fixtures::packs::operands;
use crate::domain::programs::fixtures::{Telling, framed, framed_with, held};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    EmitTarget, PlacedStep, ProgramStep, Register, RegisterCount, StepProgram, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_language::domain::node_table::types::NodeTable;
use ryo_relations::domain::relations::collections::relations::token_link::item::TokenLinkColumn;
use ryo_relations::domain::relations::types::{Column, Relation};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// What the rule's need asks of the relation's rows.
#[derive(Clone, Copy, Debug)]
enum Asked {
    /// A row holding the cell in the relation's second column, in a file where the language fills the relation.
    Cell(u64),
    /// Any row, in a file where the language fills the relation.
    AnyRow,
    /// Any row, in a file where the language refuses the relation.
    AnyRowOfNone,
}

type Input = Asked;
/// How many rows the rule found, how many steps the scan counted, and whether the file faulted (1).
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_cell_the_relations_first_row_holds_runs_the_rule"),
        word: Word::Succeeds,
        input: || Asked::Cell(0),
        expect: Expect::Value(Cow::Borrowed(&[5, 11, 0])),
    },
    Case {
        name: CaseName("a_cell_only_a_later_row_holds_runs_the_rule"),
        word: Word::Succeeds,
        input: || Asked::Cell(3),
        expect: Expect::Value(Cow::Borrowed(&[5, 11, 0])),
    },
    Case {
        name: CaseName("a_cell_no_row_holds_does_not_run_the_rule"),
        word: Word::Succeeds,
        input: || Asked::Cell(77),
        expect: Expect::Value(Cow::Borrowed(&[0, 0, 0])),
    },
    Case {
        name: CaseName("a_need_for_any_row_runs_the_rule_over_a_relation_with_rows"),
        word: Word::Succeeds,
        input: || Asked::AnyRow,
        expect: Expect::Value(Cow::Borrowed(&[5, 11, 0])),
    },
    Case {
        name: CaseName(
            "a_need_for_any_row_of_a_relation_the_language_refuses_runs_the_rule_to_fault",
        ),
        word: Word::Succeeds,
        input: || Asked::AnyRowOfNone,
        expect: Expect::Value(Cow::Borrowed(&[0, 1, 1])),
    },
];

/// The rule set: one rule, with the need, that reads every row of the relation and emits it.
fn pack(asked: Asked) -> LoweredPack {
    let steps = [
        ProgramStep::EachRow {
            source: StepRows::Filled(Relation::TokenLink),
            out: Register(0),
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: operands([]),
        },
    ];
    let value = match asked {
        Asked::Cell(cell) => Some(RowValue {
            column: Column::TokenLink(TokenLinkColumn::Target),
            value: CellWord(cell),
        }),
        Asked::AnyRow | Asked::AnyRowOfNone => None,
    };
    LoweredPack {
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Row(Relation::TokenLink, value),
                program: StepProgram(
                    steps
                        .into_iter()
                        .map(|step| PlacedStep { point: None, step })
                        .collect(),
                ),
            }],
            columns: RuleColumns::default(),
        }],
        registers: RegisterCount(2),
        ..LoweredPack::default()
    }
}

fn subject(asked: Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(asked)).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let mut held = held(2);
    let mut facts = Telling {
        relation: Relation::TokenLink,
        width: 5,
        rows: 5,
    };
    let (counted, found) = if matches!(asked, Asked::AnyRowOfNone) {
        framed(&programs.0, &mut held, &TheLoop, |frame| {
            dispatch(frame, &programs)
        })
    } else {
        framed_with(
            &NodeTable::default(),
            &mut facts,
            &programs.0,
            &mut held,
            &TheLoop,
            |frame| dispatch(frame, &programs),
        )
    };
    Ok(Cow::Owned(vec![
        u64::try_from(found.rows.len()).unwrap_or(u64::MAX),
        counted.steps.0,
        u64::from(held.state.fault.is_some()),
    ]))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
