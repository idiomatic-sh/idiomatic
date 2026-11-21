//! The `counting` service's cases: a worker that scanned nothing counted nothing, and a rule set with no unit has no ledger
//! row; a measuring worker's scans are counted per unit, files and workers add up, and the ledger names each unit's class,
//! owners, rows and steps. The rule set of the measured cases: a table of three constants (table 0); a table (table 1)
//! whose pipeline reads each constant and asks a pattern in call form for it; rule 0 reads table 1 and emits each row,
//! its two steps placed at plan points; rule 1 reads table 0 and emits each row.

use super::service::{counted, ledger, merged};
use super::types::{LedgerUnit, WorkClass};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::dispatch::service::dispatch;
use crate::domain::encoding::service::encoded;
use crate::domain::executing::entries::types::TheMeasuringLoop;
use crate::domain::executing::measuring::service::file_ended;
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::made_tables::fixtures::packs::{
    constants, definition, of, operands, pipeline, rows, scan_made, used,
};
use crate::domain::programs::fixtures::{Held, framed};
use crate::domain::programs::service::scratch_measuring;
use crate::domain::programs::types::{Programs, ProgramsEngine};
use crate::domain::scratch::service::next_file;
use crate::domain::statistics::types::PointCounts;
use crate::domain::steps::stages::types::UseMode;
use crate::domain::steps::types::{
    EmitTarget, MadeTableId, PlacedStep, PlanPoint, ProgramStep, Register, RegisterCount,
    StepProgram, StepRows,
};
use ryo_values::domain::query_plans::types::NodeDigest;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;

/// Whether the rule set holds the two rules or none, how many files each worker scans, and how many workers' counts are
/// added up.
type Input = (bool, usize, usize);
/// The files counted; then for each ledger row: what it is of (a rule's ordinal, 100 and its place for a pattern, 200 and
/// its place for a table), its class (0 navigation, 1 a fill of rows, 2 a group, 3 an ordered table, 4 a graph), its
/// owners as a sum of two to the rule's ordinal, its rows, its steps, and whether it has no idle time (1: every unit of
/// these cases gives a row in every file). Then whether rule 0's own program was timed at all (1), or 8 with no rule 0.
type Value = Cow<'static, [usize]>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_set_with_no_unit_has_no_ledger_row"),
        word: Word::Succeeds,
        input: || (false, 0, 1),
        expect: Expect::Value(Cow::Borrowed(&[0, 8])),
    },
    Case {
        name: CaseName("the_counts_of_three_workers_add_up_to_no_row_more"),
        word: Word::Succeeds,
        input: || (false, 0, 3),
        expect: Expect::Value(Cow::Borrowed(&[0, 8])),
    },
    Case {
        name: CaseName("a_measured_scan_is_counted_per_unit_with_its_class_and_owners"),
        word: Word::Succeeds,
        input: || (true, 1, 1),
        expect: Expect::Value(Cow::Borrowed(&[
            1, 0, 0, 1, 3, 4, 1, 201, 1, 1, 3, 21, 1, 202, 1, 3, 3, 0, 1, 103, 0, 1, 3, 0, 1, 1, 0,
            2, 3, 0, 1, 1,
        ])),
    },
    Case {
        name: CaseName("the_files_of_one_worker_add_up"),
        word: Word::Succeeds,
        input: || (true, 2, 1),
        expect: Expect::Value(Cow::Borrowed(&[
            2, 0, 0, 1, 6, 8, 1, 201, 1, 1, 6, 42, 1, 202, 1, 3, 6, 0, 1, 103, 0, 1, 6, 0, 1, 1, 0,
            2, 6, 0, 1, 1,
        ])),
    },
    Case {
        name: CaseName("the_counts_of_two_workers_add_up"),
        word: Word::Succeeds,
        input: || (true, 1, 2),
        expect: Expect::Value(Cow::Borrowed(&[
            2, 0, 0, 1, 6, 8, 1, 201, 1, 1, 6, 42, 1, 202, 1, 3, 6, 0, 1, 103, 0, 1, 6, 0, 1, 1, 0,
            2, 6, 0, 1, 1,
        ])),
    },
];

/// A rule that reads a table and emits each row, its steps at plan points when `placed`.
fn rule(number: u32, table: u32, placed: bool) -> LoweredRule {
    let steps = [
        ProgramStep::EachRow {
            source: StepRows::Made(MadeTableId(table)),
            out: Register(900_u32.saturating_add(number)),
        },
        ProgramStep::Emit {
            target: EmitTarget::Rule(SelectedRuleOrdinal(number)),
            columns: operands([]),
        },
    ];
    let points = (0_u32..).map(|point| placed.then_some(PlanPoint(point)));
    LoweredRule {
        rule: SelectedRuleOrdinal(number),
        node: NodeDigest([u8::try_from(number).unwrap_or(u8::MAX); 16]),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: StepProgram(
                steps
                    .into_iter()
                    .zip(points)
                    .map(|(step, point)| PlacedStep { point, step })
                    .collect(),
            ),
        }],
        columns: RuleColumns::default(),
    }
}

/// The rule set of the measured cases.
fn pack() -> LoweredPack {
    let mut stages = Vec::from(scan_made(0, 0, 1));
    stages.push(used(1, UseMode::Rows, &[5]));
    LoweredPack {
        definitions: vec![definition()],
        tables: vec![
            constants(&[1, 2, 3]),
            rows(2, pipeline(stages, [of(1), of(5)])),
        ],
        rules: vec![rule(0, 1, true), rule(1, 0, false)],
        registers: RegisterCount(1000),
        ..LoweredPack::default()
    }
}

fn subject((measured, files, workers): Input) -> Result<Value, Failure> {
    let programs = if measured {
        Programs(ProgramsEngine {
            code: encoded(&pack()).map_err(|_| ())?,
            ..ProgramsEngine::default()
        })
    } else {
        Programs::default()
    };
    let mut all = PointCounts::default();
    for _ in 0..workers {
        let mut worker = scratch_measuring(&programs);
        for _ in 0..files {
            next_file(&mut worker);
            // The loop's fixtures frame a state and a store: the worker's are lent to them for the file.
            let mut held = Held {
                state: std::mem::take(&mut worker.0.state),
                store: std::mem::take(&mut worker.0.store),
            };
            let (_counters, _found) = framed(&programs.0, &mut held, &TheMeasuringLoop, |frame| {
                let counters = dispatch(frame, &programs);
                file_ended(frame);
                counters
            });
            worker.0.state = held.state;
            worker.0.store = held.store;
        }
        merged(&mut all, &counted(&worker));
    }
    let (mut said, mut timed) = (vec![all.0.files.0], 8);
    for row in ledger(&programs, &all).0 {
        said.extend([
            match row.unit {
                LedgerUnit::Rule(rule) => usize::try_from(rule.0).unwrap_or(usize::MAX),
                LedgerUnit::Pattern(place) => place.0.saturating_add(100),
                LedgerUnit::Table(place) => place.0.saturating_add(200),
            },
            match row.class {
                WorkClass::Navigation => 0,
                WorkClass::RowsFill => 1,
                WorkClass::GroupFill => 2,
                WorkClass::OrderedFill => 3,
                WorkClass::GraphFill => 4,
            },
            row.owners
                .iter()
                .map(|rule| 1_usize << rule.0.min(16))
                .sum(),
            row.rows.0,
            row.steps.0,
            usize::from(row.idle_nanos.0 == 0),
        ]);
        if row.unit == LedgerUnit::Rule(SelectedRuleOrdinal(0)) {
            timed = usize::from(row.nanos.0 > 0);
        }
    }
    said.push(timed);
    Ok(Cow::Owned(said))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
