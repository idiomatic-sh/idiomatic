//! The scan's cases over a fixture pack, through the use case: the worker's state is reset for each file, the fill side
//! makes the table the rule reads from the rows the pack states, the loop finds the rows, and the scan answers its counts.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::encoding::service::encoded;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, Emptiness, LoweredPack, LoweredRule, MadeFill, MadeTable,
};
use crate::domain::programs::fixtures::{Collected, FILE, NoFacts};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::steps::types::{
    EmitTarget, MadeColumn, MadeTableId, PlacedStep, ProgramStep, ReadColumn, Register,
    RegisterCount, StepOperand, StepOperands, StepProgram, StepRows, TestOperation,
};
use crate::inbound::use_cases::scan::interface::ScanUseCase as _;
use crate::inbound::use_cases::scan::use_case::ScanUseCaseImpl;
use ryo_language::domain::facts::types::{CellWord, FileFacts};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::table_words::types::ColumnCount;
use std::borrow::Cow;

/// How many files the one worker scans, one after the other.
type Input = usize;
/// For each file: the cells found, then 99, the steps, candidates, rows visited, rows made and fill rows the scan counted,
/// then 98.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases: the rule emits the first cell of each row of a constant table whose second cell is above
/// 5; the table's rows are (1, 10), (2, 3) and (3, 7).
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_scan_makes_the_table_runs_the_rule_and_counts"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value(Cow::Borrowed(&[1, 3, 99, 13, 3, 3, 3, 0, 98])),
    },
    Case {
        name: CaseName("the_next_file_is_scanned_from_nothing_with_the_same_state"),
        word: Word::Succeeds,
        input: || 2,
        expect: Expect::Value(Cow::Borrowed(&[
            1, 3, 99, 13, 3, 3, 3, 0, 98, 1, 3, 99, 13, 3, 3, 3, 0, 98,
        ])),
    },
];

/// The rule set: one rule over one constant table.
fn pack() -> LoweredPack {
    let made = StepRows::Made(MadeTableId(0));
    let read = |column: u16, out: u32| ProgramStep::Read {
        source: made,
        column: ReadColumn::Made(MadeColumn(column)),
        row: Register(0),
        out: Register(out),
    };
    let steps = vec![
        ProgramStep::EachRow {
            source: made,
            out: Register(0),
        },
        read(1, 1),
        ProgramStep::Test {
            test: TestOperation::Greater,
            left: StepOperand::Register(Register(1)),
            right: StepOperand::Word(CellWord(5)),
        },
        read(0, 2),
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: StepOperands(Box::new([StepOperand::Register(Register(2))])),
        },
    ];
    let row = |first: u64, second: u64| -> Box<[CellWord]> {
        Box::new([CellWord(first), CellWord(second)])
    };
    LoweredPack {
        definitions: Vec::new(),
        tables: vec![MadeTable {
            node: NodeDigest([2; 16]),
            fill: MadeFill::Constant(ConstantRows(vec![row(1, 10), row(2, 3), row(3, 7)])),
            width: ColumnCount(2),
            indexes: Vec::new(),
            emptiness: Emptiness::Unknown,
        }],
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
            columns: RuleColumns {
                emit: ColumnKinds(vec![CellKind::Number]),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(3),
        regexes: Vec::new(),
        texts: Vec::new(),
        reads: Vec::new(),
    }
}

fn subject(files: Input) -> Result<Value, Failure> {
    let programs = Programs(ProgramsEngine {
        code: encoded(&pack()).map_err(|_| ())?,
        ..ProgramsEngine::default()
    });
    let mut scratch = scratch_for(&programs);
    let (nodes, mut all) = (NodeTable::default(), Vec::new());
    for _ in 0..files {
        let (mut facts, mut found) = (NoFacts, Collected::default());
        let file = FileFacts {
            table: &nodes,
            source: ByteSlice(b""),
            file: FILE,
            facts: &mut facts,
        };
        let counted = ScanUseCaseImpl::new(std::sync::Arc::new(MatchingUseCaseImpl))
            .scan(&programs, file, &mut scratch, &mut found)
            .map_err(|_| ())?;
        all.extend(found.rows.into_iter().flatten());
        all.extend([
            99,
            counted.steps.0,
            counted.candidates.0,
            counted.visited.0,
            counted.made_rows.0,
            counted.fill_rows.0,
            98,
        ]);
    }
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
