//! The file scan's cases, over the fixture language and the engine port's fake: a walked and scanned file gives the fixture rule's
//! rows in the report; text found in a file survives the file; a file the language refuses or fails on, and a rule's fault, are
//! reported and the run goes on; and the harness's comparison: equal rows compare, a planted difference is the first place they
//! differ, a rule the new engine refused is not compared.

use super::errors::FileScanError;
use super::service::{both, compared, harness_rows, ran, reported, scanned, worker};
use super::types::{
    ColumnKinds, EmitColumns, HarnessDifference, HarnessRows, KeptCell, RowText, RuleColumns,
    RuleFile, ScanReport, ScanRun, ScanWorker,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::engine::fixtures::fake::{EngineFake, no_kinds};
use crate::outbound::engine::interface::EnginePort as _;
use ryo_engine::domain::findings::types::FoundTarget;
use ryo_engine::domain::lowering::fixtures::named;
use ryo_engine::domain::lowering::types::{SelectedRule, SelectedRules};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::fillers::fixtures::Walking;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_language::inbound::use_cases::walk::errors::WalkError;
use ryo_language::inbound::use_cases::walk::fixtures::FAILING;
use ryo_language::inbound::use_cases::walk::types::{WalkEngine, WalkRequest};
use ryo_relations::domain::relations::types::{CellKind, Count, FileId};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;
use std::sync::Arc;

/// What a row does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Both engines found the same row of rule 0 in file 1.
    Equal,
    /// The new engine found a row the old one did not: the planted difference.
    Planted,
    /// They differ only for rule 1, which the new engine refused.
    Refused,
    /// One file of two words, scanned by one worker.
    FixtureFile,
    /// Two files whose words have the same codes and other texts, each scanned by another worker, the second file first.
    TwoFiles,
    /// A file scanned, one no language reads, one the parser fails on, one whose relation is filled in part, and one more scanned.
    Faults,
    /// The fixture file's rows against themselves as the old engine's.
    BothEqual,
    /// The fixture file's rows against the old engine's with one more, planted.
    BothPlanted,
    /// The fixture file scanned by a run that holds no columns for its rule.
    NoColumns,
    /// A file the fixture language does not read.
    Unread,
}

/// What a row saw.
#[derive(Debug, PartialEq, Eq)]
enum Seen {
    /// How many rules the harness compared.
    Compared(Count),
    /// The run's report, a line for each finding, frontier row and faulted file.
    Report(Cow<'static, str>),
}

/// Why a row was refused.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    /// The first place the engines' rows differ, without the rows.
    Differs(HarnessDifference),
    /// The fault of a file's scan.
    Fault(FileScanError),
    /// `both` stopped the run on a difference.
    Stopped,
}

type Input = Scenario;
type Value = Seen;
type Failure = Refusal;

/// The first place of a difference: rule 0 in file 1.
const FIRST_PLACE: HarnessDifference = HarnessDifference {
    at: RuleFile {
        rule: SelectedRuleOrdinal(0),
        file: FileId(1),
    },
    old: Vec::new(),
    new: Vec::new(),
};

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("equal_rows_of_the_accepted_rules_compare"),
        word: Word::Succeeds,
        input: || Scenario::Equal,
        expect: Expect::Value(Seen::Compared(Count(1))),
    },
    Case {
        name: CaseName("a_rule_the_new_engine_refused_is_not_compared"),
        word: Word::Succeeds,
        input: || Scenario::Refused,
        expect: Expect::Value(Seen::Compared(Count(1))),
    },
    Case {
        name: CaseName(
            "a_fixture_file_walked_and_scanned_gives_the_fixture_rules_rows_in_the_report",
        ),
        word: Word::Succeeds,
        input: || Scenario::FixtureFile,
        expect: Expect::Value(Seen::Report(Cow::Borrowed(
            "found file 1 rule 0: 0\t3\tone\n\
             found file 1 rule 0: 4\t7\ttwo\n\
             frontier file 1 rule 0 entry 0: one\n\
             frontier file 1 rule 0 entry 0: two",
        ))),
    },
    Case {
        name: CaseName("stored_and_frontier_text_survives_the_file"),
        word: Word::Succeeds,
        input: || Scenario::TwoFiles,
        expect: Expect::Value(Seen::Report(Cow::Borrowed(
            "found file 1 rule 0: 0\t5\talpha\n\
             found file 1 rule 0: 6\t10\tbeta\n\
             found file 2 rule 0: 0\t5\tgamma\n\
             found file 2 rule 0: 6\t11\tdelta\n\
             frontier file 1 rule 0 entry 0: alpha\n\
             frontier file 1 rule 0 entry 0: beta\n\
             frontier file 2 rule 0 entry 0: gamma\n\
             frontier file 2 rule 0 entry 0: delta",
        ))),
    },
    Case {
        name: CaseName("a_file_fault_is_reported_and_the_run_goes_on"),
        word: Word::Succeeds,
        input: || Scenario::Faults,
        expect: Expect::Value(Seen::Report(Cow::Borrowed(
            "found file 1 rule 0: 0\t3\tone\n\
             found file 5 rule 0: 0\t3\tend\n\
             frontier file 1 rule 0 entry 0: one\n\
             frontier file 5 rule 0 entry 0: end\n\
             fault file 2: file not walked: the language does not read this file\n\
             fault file 3: file not walked: the parser failed\n\
             fault file 4: rule SelectedRuleOrdinal(0): relation SyntaxError read in part: \
             PartialCause(\"a question the fixture cannot read\")",
        ))),
    },
    Case {
        name: CaseName("both_engines_equal_on_the_fixture_file_compare"),
        word: Word::Succeeds,
        input: || Scenario::BothEqual,
        expect: Expect::Value(Seen::Compared(Count(1))),
    },
    Case {
        name: CaseName("a_planted_difference_is_the_first_place_the_rows_differ"),
        word: Word::Refuses,
        input: || Scenario::Planted,
        expect: Expect::Error(Refusal::Differs(FIRST_PLACE)),
    },
    Case {
        name: CaseName("a_planted_old_row_fails_both_on_the_fixture_file"),
        word: Word::Refuses,
        input: || Scenario::BothPlanted,
        expect: Expect::Error(Refusal::Stopped),
    },
    Case {
        name: CaseName("a_found_row_that_fits_no_columns_is_a_fault_of_the_file"),
        word: Word::Refuses,
        input: || Scenario::NoColumns,
        expect: Expect::Error(Refusal::Fault(FileScanError::Columns {
            target: FoundTarget::Rule(SelectedRuleOrdinal(0)),
        })),
    },
    Case {
        name: CaseName("a_file_the_language_does_not_read_is_a_fault_of_the_file"),
        word: Word::Refuses,
        input: || Scenario::Unread,
        expect: Expect::Error(Refusal::Fault(FileScanError::Walk {
            cause: WalkError::Unsupported,
        })),
    },
];

/// Rule 0's rows in file 1, one row of `text`, and rule 1's row `other`.
fn rows(text: &str, other: &str) -> HarnessRows {
    let place = |rule| RuleFile {
        rule: SelectedRuleOrdinal(rule),
        file: FileId(1),
    };
    HarnessRows(
        [
            (place(0), vec![RowText(text.into())]),
            (place(1), vec![RowText(other.into())]),
        ]
        .into_iter()
        .collect(),
    )
}

/// The columns of the fake's rule: a finding is a start, an end and a word; its one frontier entry is a word.
fn columns() -> EmitColumns {
    EmitColumns(vec![RuleColumns {
        emit: ColumnKinds(vec![CellKind::Number, CellKind::Number, CellKind::Symbol]),
        frontier: vec![ColumnKinds(vec![CellKind::Symbol])],
    }])
}

/// The rule set of the fake's one accepted rule, the lowering fixture's one tree rule.
fn accepted(engine: &EngineFake) -> Programs {
    let plan = named("one tree rule");
    let rule = SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan: &plan,
    };
    engine
        .load(LoadSource::Plans(SelectedRules(&[rule])), &no_kinds())
        .unwrap_or_default()
}

/// The fixture language, as a linked language is held.
fn words() -> Arc<dyn WalkEngine> {
    Arc::new(Walking)
}

/// A file of the cases: its path, its number and its bytes.
fn file<'a>(path: &'a str, number: u32, bytes: &'a [u8]) -> WalkRequest<'a> {
    WalkRequest {
        path: SourcePath(path),
        file: FileId(number),
        source: ByteSlice(bytes),
    }
}

/// What one worker found in `files`, scanned in order by the fake over the fixture language.
fn worked(files: &[WalkRequest<'_>], columns: &EmitColumns) -> ScanReport {
    let engine = EngineFake::default();
    let programs = accepted(&engine);
    let mut worker = worker(&words(), &engine, &programs);
    let run = ScanRun {
        programs: &programs,
        columns,
    };
    let counted = ran(&engine, run, &mut worker, files);
    assert_eq!(
        counted.steps, counted.candidates,
        "the fake steps once per candidate"
    );
    let ScanWorker { report, .. } = worker;
    report
}

/// The fixture file: one file of two words.
fn fixture_file() -> ScanReport {
    reported([worked(&[file("a.walk", 1, b"one two")], &columns())])
}

/// A report as lines: its findings, its frontier rows, its faulted files.
fn lines(report: &ScanReport) -> Seen {
    let cells = |cells: &[KeptCell]| {
        let written: Vec<String> = cells
            .iter()
            .map(|cell| match cell {
                KeptCell::Text(word) => word.0.to_string(),
                KeptCell::Number(number) => number.0.to_string(),
            })
            .collect();
        written.join("\t")
    };
    let found = report.found.iter().map(|row| {
        let (file, rule) = (row.file.0, row.rule.0);
        format!("found file {file} rule {rule}: {}", cells(&row.cells))
    });
    let frontier = report.frontier.iter().map(|row| {
        let (file, rule, entry) = (row.file.0, row.rule.0, row.input.0);
        format!(
            "frontier file {file} rule {rule} entry {entry}: {}",
            cells(&row.cells)
        )
    });
    let faults = report
        .faults
        .iter()
        .map(|each| format!("fault file {}: {}", each.file.0, each.fault));
    let all: Vec<String> = found.chain(frontier).chain(faults).collect();
    Seen::Report(Cow::Owned(all.join("\n")))
}

/// The first place a comparison differs, without its rows.
fn first_place(difference: HarnessDifference) -> Refusal {
    Refusal::Differs(HarnessDifference {
        old: Vec::new(),
        new: Vec::new(),
        ..difference
    })
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let rule = [SelectedRuleOrdinal(0)];
    let hand = |old: &HarnessRows, new: &HarnessRows| {
        compared(old, new, &rule)
            .map(Seen::Compared)
            .map_err(first_place)
    };
    match scenario {
        Scenario::Equal => hand(&rows("a", "b"), &rows("a", "b")),
        Scenario::Refused => hand(&rows("a", "b"), &rows("a", "c")),
        Scenario::Planted => hand(&rows("a", "b"), &rows("z", "b")),
        Scenario::FixtureFile => Ok(lines(&fixture_file())),
        Scenario::TwoFiles => {
            let (first, second) = (
                worked(&[file("b.walk", 2, b"gamma delta")], &columns()),
                worked(&[file("a.walk", 1, b"alpha beta")], &columns()),
            );
            Ok(lines(&reported([first, second])))
        }
        Scenario::Faults => {
            let files = [
                file("a.walk", 1, b"one"),
                file("b.txt", 2, b"two"),
                file("c.walk", 3, FAILING),
                file("d.walk", 4, b"why?"),
                file("e.walk", 5, b"end"),
            ];
            Ok(lines(&reported([worked(&files, &columns())])))
        }
        Scenario::BothEqual | Scenario::BothPlanted => {
            let report = fixture_file();
            let mut old = harness_rows(&report);
            if matches!(scenario, Scenario::BothPlanted) {
                for found in old.0.values_mut() {
                    found.push(RowText("planted".into()));
                }
            }
            both(&old, &report, &rule)
                .map(Seen::Compared)
                .map_err(|_| Refusal::Stopped)
        }
        Scenario::NoColumns | Scenario::Unread => {
            let engine = EngineFake::default();
            let (programs, held) = (accepted(&engine), EmitColumns::default());
            let mut worker = worker(&words(), &engine, &programs);
            let run = ScanRun {
                programs: &programs,
                columns: &held,
            };
            let path = match scenario {
                Scenario::Unread => "a.txt",
                _ => "a.walk",
            };
            scanned(&engine, run, &mut worker, file(path, 1, b"one two"))
                .map(|_| lines(&worker.report))
                .map_err(Refusal::Fault)
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
