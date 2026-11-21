//! The closure's cases, through the loop, over a fixture pack written as steps and encoded by the straight encoding. The
//! pack states a table of edges (from, to): 1 to 2, 2 to 3, 3 to 1, 3 to 4, and 1 to 2 once more; the rule reads, for each
//! seed of a table of seeds, every element reached over one or more edges, and emits the seed and the element. The edges
//! of an element are an inline body: the rows of the table of edges by their first column.

use super::service::closure;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{CodeEntry, Op};
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheLoop;
use crate::domain::executing::service::run as run_branch;
use crate::domain::findings::types::{ColumnKinds, RuleColumns};
use crate::domain::lowered::types::{
    Branch, ConstantRows, Emptiness, LoweredPack, LoweredRule, MadeFill,
};
use crate::domain::made_tables::fixtures::packs::{constants, of, table};
use crate::domain::programs::fixtures::{framed, held};
use crate::domain::programs::frames::types::RunMode;
use crate::domain::programs::operands::service::set;
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::steps::types::{
    EmitTarget, MadeColumn, MadeTableId, MatchColumns, PlacedStep, ProgramStep, ReadColumn,
    Register, RegisterCount, RegisterList, StepBody, StepOperands, StepProgram, StepRows,
};
use ryo_language::domain::facts::types::CellWord;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// The seeds, in order.
type Input = &'static [u64];
/// The seed and the element of each row found; then 99, the rows the run answered, the rows visited, the words counted
/// as not run, and how many cursors and staged cells are left; then, the word asked once more for the first seed, the
/// rows it answered and the cells it left staged.
type Value = Cow<'static, [u64]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_closure_reaches_each_element_once_through_a_cycle_and_its_seed_when_an_edge_gives_it"),
        word: Word::Succeeds,
        input: || &[1],
        expect: Expect::Value(Cow::Borrowed(&[1, 2, 1, 3, 1, 1, 1, 4, 99, 4, 19, 0, 0, 0, 4, 4])),
    },
    Case {
        name: CaseName("an_element_with_no_edge_reaches_nothing"),
        word: Word::Succeeds,
        input: || &[4, 5],
        expect: Expect::Value(Cow::Borrowed(&[99, 0, 2, 0, 0, 0, 0, 0])),
    },
    Case {
        name: CaseName("each_seed_has_its_own_closure"),
        word: Word::Succeeds,
        input: || &[3, 2],
        expect: Expect::Value(Cow::Borrowed(&[
            3, 1, 3, 4, 3, 2, 3, 3, 2, 3, 2, 1, 2, 4, 2, 2, 99, 8, 36, 0, 0, 0, 4, 4,
        ])),
    },
];

/// A program of steps with no plan point.
fn program(steps: Vec<ProgramStep>) -> StepProgram {
    StepProgram(
        steps
            .into_iter()
            .map(|step| PlacedStep { point: None, step })
            .collect(),
    )
}

/// The rule set: the edges (table 0), the seeds (table 1), one rule.
fn pack(seeds: Input) -> LoweredPack {
    let edges = StepRows::Made(MadeTableId(0));
    let edge = |from: u64, to: u64| -> Box<[CellWord]> { Box::new([CellWord(from), CellWord(to)]) };
    let reached = StepBody::Inline(program(vec![
        ProgramStep::EachMatch {
            source: edges,
            columns: MatchColumns(Box::new([ReadColumn::Made(MadeColumn(0))])),
            values: StepOperands(Box::new([of(5)])),
            out: Register(6),
        },
        ProgramStep::Read {
            source: edges,
            column: ReadColumn::Made(MadeColumn(1)),
            row: Register(6),
            out: Register(7),
        },
        ProgramStep::Yield {
            row: RegisterList(Box::new([Register(7)])),
        },
    ]));
    let steps = vec![
        ProgramStep::EachRow {
            source: StepRows::Made(MadeTableId(1)),
            out: Register(0),
        },
        ProgramStep::Read {
            source: StepRows::Made(MadeTableId(1)),
            column: ReadColumn::Made(MadeColumn(0)),
            row: Register(0),
            out: Register(1),
        },
        ProgramStep::Closure {
            edges: reached,
            from: Register(5),
            seed: Register(1),
            out: Register(2),
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(0)),
            columns: StepOperands(Box::new([of(1), of(2)])),
        },
    ];
    LoweredPack {
        tables: vec![
            table(
                90,
                2,
                MadeFill::Constant(ConstantRows(vec![
                    edge(1, 2),
                    edge(2, 3),
                    edge(3, 1),
                    edge(3, 4),
                    edge(1, 2),
                ])),
            ),
            constants(seeds),
        ],
        rules: vec![LoweredRule {
            rule: SelectedRuleOrdinal(0),
            node: NodeDigest([1; 16]),
            branches: vec![Branch {
                emptiness: Emptiness::Unknown,
                program: program(steps),
            }],
            columns: RuleColumns {
                emit: ColumnKinds(vec![CellKind::Number, CellKind::Number]),
                frontier: Vec::new(),
            },
        }],
        registers: RegisterCount(8),
        ..LoweredPack::default()
    }
}

fn subject(seeds: Input) -> Result<Value, Failure> {
    let code = encoded(&pack(seeds)).map_err(|_| ())?;
    let entry = code
        .rules
        .first()
        .and_then(|rule| rule.branches.first())
        .map(|branch| branch.entry)
        .ok_or(())?;
    let programs = ProgramsEngine {
        code,
        ..ProgramsEngine::default()
    };
    let mut held = held(8);
    let (rows, found) = framed(&programs, &mut held, &TheLoop, |frame| {
        run_branch(frame, entry, RunMode::Rows)
    });
    let mut all: Vec<u64> = found.rows.into_iter().flatten().collect();
    all.extend([
        99,
        rows.0,
        held.state.work.visited.0,
        held.state.work.unbuilt.0,
        u64::try_from(held.state.frames.len()).unwrap_or(u64::MAX),
        u64::try_from(held.state.staged.len()).unwrap_or(u64::MAX),
    ]);
    // Then the word asked once more, outside a run, for the first seed: what it leaves staged for its item.
    let first = seeds.first().copied().unwrap_or(0);
    let (asked, _found) = framed(&programs, &mut held, &TheLoop, |frame| {
        let walking = frame
            .fill
            .programs
            .code
            .words
            .iter()
            .copied()
            .enumerate()
            .find(|(_, word)| word.op == Op::Closure);
        set(frame, Register(1), CellWord(first));
        if let Some((place, word)) = walking {
            closure(
                frame,
                word,
                CodeEntry(u32::try_from(place).unwrap_or(u32::MAX)),
            );
        }
        [
            u64::try_from(frame.state.answer.rows.0).unwrap_or(u64::MAX),
            u64::try_from(frame.state.staged.len()).unwrap_or(u64::MAX),
        ]
    });
    all.extend(asked);
    Ok(Cow::Owned(all))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
