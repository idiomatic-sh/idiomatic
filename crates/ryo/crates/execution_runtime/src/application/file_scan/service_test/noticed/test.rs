//! The cases of what the file scan keeps beside a file's rows: each notice the engine gives of a rule's work, with its file,
//! while the rule's rows stay complete; and what the scans counted, a worker's and the run's. Over the engine port's fake,
//! told to notice in one file, and the fixture language.

use crate::application::file_scan::service::{emit_columns, place_columns, ran, reported, worker};
use crate::application::file_scan::types::{ScanMeasuring, ScanReport, ScanRun, ScanWorker};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::fixtures::fake::{EngineFake, no_kinds};
use crate::outbound::engine::interface::EnginePort as _;
use ryo_engine::domain::findings::types::{
    HeaviestWork, InstructionCount, WorkNotice, WorkVerdict,
};
use ryo_engine::domain::lowering::fixtures::{digests_of, named};
use ryo_engine::domain::lowering::types::{SelectedRule, SelectedRules};
use ryo_engine::domain::statistics::types::StatisticsBytes;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::types::{WalkEngine, WalkRequest};
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;
use std::sync::Arc;

/// What a row scans.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Two files of two words, one worker, the engine noticing the rule in the second.
    NoticedInOneFile,
    /// The same two files, the engine noticing nothing.
    NothingNoticed,
    /// The same two files, a worker each, the second file's worker first, the engine noticing the rule in both files.
    TwoWorkers,
}

type Input = Scenario;
/// The report said: how many rows it found, each notice, and what its scans counted.
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rules_notice_is_kept_with_its_file_and_its_rows_are_complete"),
        word: Word::Succeeds,
        input: || Scenario::NoticedInOneFile,
        expect: Expect::Value(Cow::Borrowed(
            "4 rows; notices [file 2 rule 0 Over work 1000000 allowance 128000 step 3 Product { left: InstructionCount(1000), right: InstructionCount(1000) }]; counted steps 4 candidates 4 made 0 filled 0 visited 4",
        )),
    },
    Case {
        name: CaseName("a_run_the_engine_notices_nothing_in_keeps_no_notice"),
        word: Word::Succeeds,
        input: || Scenario::NothingNoticed,
        expect: Expect::Value(Cow::Borrowed(
            "4 rows; notices []; counted steps 4 candidates 4 made 0 filled 0 visited 4",
        )),
    },
    Case {
        name: CaseName("the_runs_report_holds_its_workers_notices_by_file_and_what_they_counted"),
        word: Word::Succeeds,
        input: || Scenario::TwoWorkers,
        expect: Expect::Value(Cow::Borrowed(
            "4 rows; notices [file 1 rule 0 Over work 1000000 allowance 128000 step 3 Product { left: InstructionCount(1000), right: InstructionCount(1000) }; file 2 rule 0 Over work 1000000 allowance 128000 step 3 Product { left: InstructionCount(1000), right: InstructionCount(1000) }]; counted steps 4 candidates 4 made 0 filled 0 visited 4",
        )),
    },
];

/// The notice the fake gives: a join with no key of 1000 rows with 1000.
fn notice() -> WorkNotice {
    WorkNotice {
        rule: SelectedRuleOrdinal(9),
        verdict: WorkVerdict::Over,
        work: InstructionCount(1_000_000),
        allowance: InstructionCount(128_000),
        step: StepId(3),
        heaviest: HeaviestWork::Product {
            left: InstructionCount(1000),
            right: InstructionCount(1000),
        },
    }
}

/// What one worker keeps of `files`, scanned by the fake's one tree rule; the fake notices the rule in the files `noticed`
/// names, one fake a file since a fake notices in one.
fn worked(files: &[(u32, &[u8])], noticed: &[u32]) -> ScanReport {
    let plan = named("one tree rule");
    let digests = digests_of(&plan, RuleBindings(&[]));
    let mut reports = Vec::new();
    for (number, bytes) in files {
        let engine = if noticed.contains(number) {
            EngineFake::noticing(FileId(*number), notice())
        } else {
            EngineFake::default()
        };
        let rule = SelectedRule {
            rule: SelectedRuleOrdinal(0),
            plan: &plan,
            bindings: RuleBindings(&[]),
            digests: &digests,
        };
        let source = LoadSource::Plans(SelectedRules(&[rule]), StatisticsBytes(&[]));
        let programs = engine
            .load(source, &no_kinds())
            .map(|loaded| loaded.programs)
            .unwrap_or_default();
        let words: Arc<dyn WalkEngine> = Arc::new(Walking);
        let mut worker = worker(&words, &engine, &programs, ScanMeasuring::Plain);
        let (columns, places) = (
            emit_columns(&engine, &programs),
            place_columns(&[SelectedRuleOrdinal(0)], std::slice::from_ref(&plan.emit)),
        );
        let run = ScanRun {
            programs: &programs,
            columns: &columns,
            places: &places,
        };
        let file = WalkRequest {
            path: SourcePath("a.walk"),
            file: FileId(*number),
            source: ByteSlice(bytes),
        };
        let _counted = ran(&engine, &run, &mut worker, &[file]);
        let ScanWorker { report, .. } = worker;
        reports.push(report);
    }
    reported(reports)
}

/// A report said.
fn said(report: &ScanReport) -> Value {
    let notices: Vec<String> = report
        .notices
        .iter()
        .map(|each| {
            let WorkNotice {
                rule,
                verdict,
                work,
                allowance,
                step,
                heaviest,
            } = each.notice;
            format!(
                "file {} rule {} {verdict:?} work {} allowance {} step {} {heaviest:?}",
                each.file.0, rule.0, work.0, allowance.0, step.0
            )
        })
        .collect();
    let counted = report.counted;
    Cow::Owned(format!(
        "{} rows; notices [{}]; counted steps {} candidates {} made {} filled {} visited {}",
        report.found.len(),
        notices.join("; "),
        counted.steps.0,
        counted.candidates.0,
        counted.made_rows.0,
        counted.fill_rows.0,
        counted.visited.0
    ))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let (first, second): ((u32, &[u8]), (u32, &[u8])) = ((1, b"one two"), (2, b"three four"));
    Ok(match scenario {
        Scenario::NoticedInOneFile => said(&worked(&[first, second], &[2])),
        Scenario::NothingNoticed => said(&worked(&[first, second], &[])),
        Scenario::TwoWorkers => {
            let (late, early) = (worked(&[second], &[2]), worked(&[first], &[1]));
            said(&reported([late, early]))
        }
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
