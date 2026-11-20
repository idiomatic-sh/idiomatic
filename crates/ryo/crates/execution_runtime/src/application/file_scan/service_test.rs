//! The file scan's cases, over the fixture language and the engine port's fake: a walked and scanned file gives the fixture rule's
//! rows in the report; text found in a file survives the file; a row is placed by the first of its rule's key columns, then its
//! subject, that is a stretch of the file or a node of it the parser did not imply; a file the language refuses or fails on, and a rule's fault, are
//! reported and the run goes on.

use super::errors::FileScanError;
use super::service::{crossed, emit_columns, place_columns, ran, reported, taken, worker};
use super::types::{EmitColumns, PlaceColumns, ScanReport, ScanRun, ScanWorker};
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
use ryo_relations::domain::labels::types::NameRef;
use ryo_relations::domain::relations::service::{column_named, columns};
use ryo_relations::domain::relations::types::{FileId, Relation};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Heading, Input as StepInput};
use ryo_values::domain::plans::types::Operation;
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::scan_rows::types::{KeptCell, RowPlace};
use std::borrow::Cow;
use std::sync::Arc;

/// What a row does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// One file of two words, scanned by one worker.
    FixtureFile,
    /// Two files whose words have the same codes and other texts, each scanned by another worker, the second file first.
    TwoFiles,
    /// A file scanned, one no language reads, one the parser fails on, one whose relation is filled in part, and one more scanned.
    Faults,
    /// A rule of the one tree rule's shape that keeps the element, the file, the span and the text, keyed by the element.
    KeyedByNode,
    /// The same rule keyed by the element, over a file whose first word the writer left out: an implied node.
    KeyedByImpliedNode,
    /// The same rule keyed by the span.
    KeyedBySpan,
    /// The same rule keyed by the text, its subject the file: no column places its rows.
    KeyedByText,
    /// Three files taken one at a time: two words out of order, a file no language reads, a file read in part.
    Taken,
    /// The fixture file scanned by a run that holds no columns for its rule.
    NoColumns,
    /// A file the fixture language does not read.
    Unread,
}

type Input = Scenario;
/// The run's report, a line for each finding, frontier row and faulted file.
type Value = Cow<'static, str>;
/// The fault of a file's scan.
type Failure = FileScanError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "a_fixture_file_walked_and_scanned_gives_the_fixture_rules_rows_in_the_report",
        ),
        word: Word::Succeeds,
        input: || Scenario::FixtureFile,
        expect: Expect::Value(Cow::Borrowed(
            "found file 1 rule 0 at nowhere: 0\t3\tone\nfound file 1 rule 0 at nowhere: 4\t7\ttwo\nfrontier file 1 rule 0 entry 0: one\nfrontier file 1 rule 0 entry 0: two",
        )),
    },
    Case {
        name: CaseName("stored_and_frontier_text_survives_the_file"),
        word: Word::Succeeds,
        input: || Scenario::TwoFiles,
        expect: Expect::Value(Cow::Borrowed(
            "found file 1 rule 0 at nowhere: 0\t5\talpha\nfound file 1 rule 0 at nowhere: 6\t10\tbeta\nfound file 2 rule 0 at nowhere: 0\t5\tgamma\nfound file 2 rule 0 at nowhere: 6\t11\tdelta\nfrontier file 1 rule 0 entry 0: alpha\nfrontier file 1 rule 0 entry 0: beta\nfrontier file 2 rule 0 entry 0: gamma\nfrontier file 2 rule 0 entry 0: delta",
        )),
    },
    Case {
        name: CaseName("a_file_fault_is_reported_and_the_run_goes_on"),
        word: Word::Succeeds,
        input: || Scenario::Faults,
        expect: Expect::Value(Cow::Borrowed(
            "found file 1 rule 0 at nowhere: 0\t3\tone\nfound file 5 rule 0 at nowhere: 0\t3\tend\nfrontier file 1 rule 0 entry 0: one\nfrontier file 5 rule 0 entry 0: end\nfault file 2: file not walked: the language does not read this file\nfault file 3: file not walked: the parser failed\nfault file 4: rule SelectedRuleOrdinal(0): relation SyntaxError read in part: PartialCause(\"a question the fixture cannot read\")",
        )),
    },
    Case {
        name: CaseName("a_row_keyed_by_a_node_is_placed_where_the_node_is"),
        word: Word::Succeeds,
        input: || Scenario::KeyedByNode,
        expect: Expect::Value(Cow::Borrowed(
            "found file 1 rule 0 at 0..3: 4294967297\t1\t3\ttwo\nfound file 1 rule 0 at 0..7: 4294967296\t1\t7\t\nfound file 1 rule 0 at 4..7: 4294967298\t1\t17179869191\tone",
        )),
    },
    Case {
        name: CaseName("a_row_keyed_by_an_implied_node_has_no_place"),
        word: Word::Succeeds,
        input: || Scenario::KeyedByImpliedNode,
        expect: Expect::Value(Cow::Borrowed(
            "found file 1 rule 0 at nowhere: 4294967297\t1\t0\t\nfound file 1 rule 0 at 0..1: 4294967298\t1\t1\t~\nfound file 1 rule 0 at 0..5: 4294967296\t1\t5\t\nfound file 1 rule 0 at 2..5: 4294967299\t1\t8589934597\tone",
        )),
    },
    Case {
        name: CaseName("a_row_keyed_by_a_span_is_placed_at_the_span"),
        word: Word::Succeeds,
        input: || Scenario::KeyedBySpan,
        expect: Expect::Value(Cow::Borrowed(
            "found file 1 rule 0 at 0..3: 4294967297\t1\t3\ttwo\nfound file 1 rule 0 at 0..7: 4294967296\t1\t7\t\nfound file 1 rule 0 at 4..7: 4294967298\t1\t17179869191\tone",
        )),
    },
    Case {
        name: CaseName("a_row_no_column_places_has_no_place"),
        word: Word::Succeeds,
        input: || Scenario::KeyedByText,
        expect: Expect::Value(Cow::Borrowed(
            "found file 1 rule 0 at nowhere: 4294967296\t1\t7\t\nfound file 1 rule 0 at nowhere: 4294967297\t1\t3\ttwo\nfound file 1 rule 0 at nowhere: 4294967298\t1\t17179869191\tone",
        )),
    },
    Case {
        name: CaseName("a_file_taken_hands_its_rows_sorted_and_its_fault_as_a_sentence"),
        word: Word::Succeeds,
        input: || Scenario::Taken,
        expect: Expect::Value(Cow::Borrowed(
            "file 1: [\"\", \"two\", \"one\"]; left in the worker 0; fault None\nfile 2: []; left in the worker 0; fault Some((NoLanguage, \"file not walked: the language does not read this file\"))\nfile 3: []; left in the worker 0; fault Some((Rule(SelectedRuleOrdinal(0)), \"rule SelectedRuleOrdinal(0): relation SyntaxError read in part: PartialCause(\\\"a question the fixture cannot read\\\")\"))",
        )),
    },
    Case {
        name: CaseName("a_found_row_that_fits_no_columns_is_a_fault_of_the_file"),
        word: Word::Refuses,
        input: || Scenario::NoColumns,
        expect: Expect::Error(FileScanError::Columns {
            target: FoundTarget::Rule(SelectedRuleOrdinal(0)),
        }),
    },
    Case {
        name: CaseName("a_file_the_language_does_not_read_is_a_fault_of_the_file"),
        word: Word::Refuses,
        input: || Scenario::Unread,
        expect: Expect::Error(FileScanError::Walk {
            cause: WalkError::Unsupported,
        }),
    },
];

/// The place of a column of the tree's relation among its columns.
fn column(name: &str) -> ColumnIndex {
    let relation = Relation::SyntaxTree;
    let place = column_named(relation, NameRef(name))
        .and_then(|named| columns(relation).iter().position(|each| *each == named));
    ColumnIndex(u32::try_from(place.unwrap_or(usize::MAX)).unwrap_or(u32::MAX))
}

/// A rule of the one tree rule's shape as a compile leaves it: one step keeping the element, the file, the span and the text of
/// the tree's relation, its subject the file and its key the kept column `key`.
fn keeping(key: u32) -> Plan {
    let mut plan = named("one tree rule");
    let names = ["id", "file", "span", "text"];
    if let Some(step) = plan.steps.first_mut() {
        step.operation = Operation::Select {
            from: StepInput::Relation(Relation::SyntaxTree),
            comparisons: Vec::new(),
            kept: names.iter().map(|name| column(name)).collect(),
        };
    }
    plan.emit.subject = ColumnIndex(1);
    plan.emit.key = vec![ColumnIndex(key)];
    plan.emit.headings = names.iter().map(|name| Heading((*name).into())).collect();
    plan
}

/// The rule set of the fake's one accepted rule, `plan`.
fn accepted(engine: &EngineFake, plan: &Plan) -> Programs {
    let rule = SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan,
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

/// What one worker found in `files`, scanned in order by the fake's rule of `plan` over the fixture language.
fn worked(plan: &Plan, files: &[WalkRequest<'_>]) -> ScanReport {
    let engine = EngineFake::default();
    let programs = accepted(&engine, plan);
    let mut worker = worker(&words(), &engine, &programs);
    let (columns, places) = (
        emit_columns(&engine, &programs),
        place_columns(&[SelectedRuleOrdinal(0)], std::slice::from_ref(&plan.emit)),
    );
    let run = ScanRun {
        programs: &programs,
        columns: &columns,
        places: &places,
    };
    let counted = ran(&engine, &run, &mut worker, files);
    assert_eq!(
        counted.steps, counted.candidates,
        "the fake steps once per candidate"
    );
    let ScanWorker { report, .. } = worker;
    report
}

/// A report as lines: its findings, its frontier rows, its faulted files.
fn lines(report: &ScanReport) -> Value {
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
        let place = match row.place {
            RowPlace::Unplaced => "nowhere".to_owned(),
            RowPlace::At(span) => format!("{}..{}", span.start.0, span.end.0),
        };
        format!(
            "found file {file} rule {rule} at {place}: {}",
            cells(&row.cells)
        )
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
    Cow::Owned(all.join("\n"))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let tree = named("one tree rule");
    match scenario {
        Scenario::FixtureFile => Ok(lines(&reported([worked(
            &tree,
            &[file("a.walk", 1, b"one two")],
        )]))),
        Scenario::TwoFiles => {
            let (first, second) = (
                worked(&tree, &[file("b.walk", 2, b"gamma delta")]),
                worked(&tree, &[file("a.walk", 1, b"alpha beta")]),
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
            Ok(lines(&reported([worked(&tree, &files)])))
        }
        Scenario::KeyedByNode
        | Scenario::KeyedByImpliedNode
        | Scenario::KeyedBySpan
        | Scenario::KeyedByText => {
            let key = match scenario {
                Scenario::KeyedByNode | Scenario::KeyedByImpliedNode => 0,
                Scenario::KeyedBySpan => 2,
                _ => 3,
            };
            let text: &[u8] = match scenario {
                Scenario::KeyedByImpliedNode => b"~ one",
                _ => b"two one",
            };
            let files = [file("a.walk", 1, text)];
            Ok(lines(&reported([worked(&keeping(key), &files)])))
        }
        Scenario::Taken => {
            let (engine, plan) = (EngineFake::default(), keeping(3));
            let programs = accepted(&engine, &plan);
            let mut worker = worker(&words(), &engine, &programs);
            let (columns, places) = (
                emit_columns(&engine, &programs),
                place_columns(&[SelectedRuleOrdinal(0)], std::slice::from_ref(&plan.emit)),
            );
            let run = ScanRun {
                programs: &programs,
                columns: &columns,
                places: &places,
            };
            let files = [
                file("a.walk", 1, b"two one"),
                file("b.txt", 2, b"two"),
                file("c.walk", 3, b"why?"),
            ];
            let (mut rows, mut said) = (Vec::new(), Vec::new());
            for each in files {
                let before = rows.len();
                let fault = taken(&engine, &run, &mut worker, each, &mut rows).err();
                let texts: Vec<String> = rows
                    .iter()
                    .skip(before)
                    .filter_map(|row| match row.cells.last() {
                        Some(KeptCell::Text(word)) => Some(word.0.to_string()),
                        Some(KeptCell::Number(_)) | None => None,
                    })
                    .collect();
                let fault = fault.map(|fault| crossed(&fault));
                said.push(format!(
                    "file {}: {texts:?}; left in the worker {}; fault {:?}",
                    each.file.0,
                    worker.report.found.len(),
                    fault.map(|fault| (fault.of, fault.sentence.0)),
                ));
            }
            Ok(Cow::Owned(said.join("\n")))
        }
        Scenario::NoColumns | Scenario::Unread => {
            let engine = EngineFake::default();
            let programs = accepted(&engine, &tree);
            let mut worker = worker(&words(), &engine, &programs);
            // The run of a host that holds no columns for the rule.
            let (held, none) = (emit_columns(&engine, &programs), EmitColumns::default());
            let places = PlaceColumns::default();
            let run = ScanRun {
                programs: &programs,
                columns: match scenario {
                    Scenario::Unread => &held,
                    _ => &none,
                },
                places: &places,
            };
            let path = match scenario {
                Scenario::Unread => "a.txt",
                _ => "a.walk",
            };
            let _counted = ran(&engine, &run, &mut worker, &[file(path, 1, b"one two")]);
            match worker.report.faults.pop() {
                Some(faulted) => Err(faulted.fault),
                None => Ok(lines(&worker.report)),
            }
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
