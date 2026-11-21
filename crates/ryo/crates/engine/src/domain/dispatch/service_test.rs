//! The `dispatch` service's cases: every rule whose need holds runs, in rule order; a rule whose anchor kind is absent, or
//! whose table has no row, costs no candidate and no step. The budget's charge of fills and the fill counters are the
//! cases of `fills`.

mod fills;
mod oversized;
mod presence;

use super::service::dispatch;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable,
};
use crate::domain::programs::fixtures::{framed, held, table};
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    AnchorKinds, EmitTarget, MadeColumn, MadeTableId, PlacedStep, ProgramStep, ReadColumn,
    Register, RegisterCount, StepOperand, StepOperands, StepProgram, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::{CellKind, KindBits};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// The rules a case loads, each by what its branch waits on. Every rule walks table 0, of rows (5) and (6), and emits its
/// own number and the row's cell; table 1 has no row; the file has no node.
#[derive(Clone, Copy, Debug)]
enum Waits {
    /// Nothing known.
    Unknown,
    /// A node of kind 3.
    Kind,
    /// A row of table 1.
    EmptyTable,
    /// A row of table 0.
    FullTable,
}

type Input = &'static [Waits];
/// The cells found; then 99, the steps, the candidates and the rows visited the scan counted.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_rules_whose_need_holds_run_in_rule_order"),
        word: Word::Succeeds,
        input: || {
            &[
                Waits::Unknown,
                Waits::Kind,
                Waits::EmptyTable,
                Waits::FullTable,
            ]
        },
        expect: Expect::Value(Cow::Borrowed(&[0, 5, 0, 6, 3, 5, 3, 6, 99, 14, 4, 4])),
    },
    Case {
        name: CaseName("a_file_without_a_rules_anchor_costs_that_rule_no_step"),
        word: Word::Succeeds,
        input: || &[Waits::Kind],
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 0, 0])),
    },
    Case {
        name: CaseName("a_rule_whose_table_has_no_row_costs_no_step"),
        word: Word::Succeeds,
        input: || &[Waits::EmptyTable],
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 0, 0])),
    },
];

/// A made table of one column, its rows written by the case.
fn made(node: u8) -> MadeTable {
    MadeTable {
        node: NodeDigest([node; 16]),
        fill: MadeFill::Constant(ConstantRows(Vec::new())),
        width: ColumnCount(1),
        indexes: Vec::new(),
        emptiness: Emptiness::Unknown,
    }
}

/// The rule of a number: it walks table 0 and emits its number and the row's cell.
fn rule(number: u32, waits: Waits) -> LoweredRule {
    let source = StepRows::Made(MadeTableId(0));
    let steps = vec![
        ProgramStep::EachRow {
            source,
            out: Register(0),
        },
        ProgramStep::Read {
            source,
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(0),
            out: Register(1),
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
            columns: StepOperands(Box::new([
                StepOperand::Word(CellWord(u64::from(number))),
                StepOperand::Register(Register(1)),
            ])),
        },
    ];
    LoweredRule {
        rule: SelectedRuleOrdinal(number),
        node: NodeDigest([u8::try_from(number).unwrap_or(u8::MAX).wrapping_add(10); 16]),
        branches: vec![Branch {
            emptiness: match waits {
                Waits::Unknown => Emptiness::Unknown,
                Waits::Kind => Emptiness::Kind(AnchorKinds(Box::new([KindBits(3)]))),
                Waits::EmptyTable => Emptiness::Table(MadeTableId(1)),
                Waits::FullTable => Emptiness::Table(MadeTableId(0)),
            },
            program: StepProgram(
                steps
                    .into_iter()
                    .map(|step| PlacedStep { point: None, step })
                    .collect(),
            ),
        }],
        columns: RuleColumns {
            emit: ColumnKinds(vec![CellKind::Number, CellKind::Number]),
            frontier: Vec::new(),
        },
    }
}

fn subject(rules: Input) -> Result<Value, Failure> {
    let pack = LoweredPack {
        definitions: Vec::new(),
        tables: vec![made(2), made(3)],
        rules: rules
            .iter()
            .enumerate()
            .map(|(number, waits)| rule(u32::try_from(number).unwrap_or(u32::MAX), *waits))
            .collect(),
        registers: RegisterCount(2),
        regexes: Vec::new(),
        texts: Vec::new(),
        reads: Vec::new(),
    };
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let mut held = held(2);
    table(&mut held, 0, 1, &[&[5], &[6]]);
    table(&mut held, 1, 1, &[]);
    let (counted, found) = framed(&programs.0, &mut held, &TheLoop, |frame| {
        dispatch(frame, &programs)
    });
    let mut all: Vec<u64> = found.rows.into_iter().flatten().collect();
    all.extend([99, counted.steps.0, counted.candidates.0, counted.visited.0]);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
