//! The cases of `dispatch` over reads by key of one table of constants (the cells 5, 5 and 6, indexed by their one
//! column) by a number of rules, each of which reads the rows whose cell is 5 and, for each, the rows whose cell is
//! that row's, and emits each pair, in a number of files scanned by one worker. A rule that read a site before in the file asks neither the table's need nor its charge
//! again; another rule, and the same rule in the next file, do.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{
    Branch, ConstantRows, Emptiness, LoweredPack, LoweredRule, MadeFill,
};
use crate::domain::made_tables::fixtures::packs::{of, operands, table, word};
use crate::domain::programs::fixtures::{Held, framed};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::scratch::service::next_file;
use crate::domain::steps::types::{
    EmitTarget, MadeColumn, MadeTableId, MatchColumns, PlacedStep, ProgramStep, ReadColumn,
    Register, RegisterCount, StepOperand, StepProgram, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// How many rules read the table, and how many files the one worker scans.
type Input = (u32, usize);
/// For each file: the rows found, and the terms of the last rule's allowance beyond its candidates and keys (the rows
/// into and out of the table: 3).
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_that_reads_a_site_twice_is_charged_its_table_once"),
        word: Word::Succeeds,
        input: || (1, 1),
        expect: Expect::Value(Cow::Borrowed(&[4, 3])),
    },
    Case {
        name: CaseName("a_second_rule_that_reads_the_site_is_charged_the_table_too"),
        word: Word::Succeeds,
        input: || (2, 1),
        expect: Expect::Value(Cow::Borrowed(&[8, 3])),
    },
    Case {
        name: CaseName("the_next_file_charges_the_rule_again"),
        word: Word::Succeeds,
        input: || (1, 2),
        expect: Expect::Value(Cow::Borrowed(&[4, 3, 4, 3])),
    },
];

/// The rule set: the table, and the rules.
fn pack(rules: u32) -> LoweredPack {
    let matched = |value: StepOperand, out: u32| ProgramStep::EachMatch {
        source: StepRows::Made(MadeTableId(0)),
        columns: MatchColumns(Box::new([ReadColumn::Made(MadeColumn(0))])),
        values: operands([value]),
        out: Register(out),
    };
    let rule = |number: u32| LoweredRule {
        rule: SelectedRuleOrdinal(number),
        node: NodeDigest([u8::try_from(number).unwrap_or(u8::MAX); 16]),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: StepProgram(
                [
                    matched(word(5), 0),
                    ProgramStep::Read {
                        source: StepRows::Made(MadeTableId(0)),
                        column: ReadColumn::Made(MadeColumn(0)),
                        row: Register(0),
                        out: Register(2),
                    },
                    matched(of(2), 1),
                    ProgramStep::Emit {
                        target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
                        columns: operands([]),
                    },
                ]
                .into_iter()
                .map(|step| PlacedStep { point: None, step })
                .collect(),
            ),
        }],
        columns: RuleColumns::default(),
    };
    let cell = |held: u64| -> Box<[CellWord]> { Box::new([CellWord(held)]) };
    LoweredPack {
        tables: vec![table(
            90,
            1,
            MadeFill::Constant(ConstantRows(vec![cell(5), cell(5), cell(6)])),
        )],
        rules: (0..rules).map(rule).collect(),
        registers: RegisterCount(4),
        ..LoweredPack::default()
    }
}

fn subject((rules, files): Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack(rules)).map_err(|_| ())?,
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
        let (_counted, found) = framed(&programs.0, &mut held, &TheLoop, |frame| {
            dispatch(frame, &programs)
        });
        said.extend([
            u64::try_from(found.rows.len()).unwrap_or(u64::MAX),
            held.state.budget.terms.0,
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
