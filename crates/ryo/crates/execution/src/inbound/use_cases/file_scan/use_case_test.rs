//! The file scan use case's cases, over the engine port's fake and the fixture language: the old engine alone loads nothing; the
//! new engine's rule set scans a file in a worker and hands its rows, placed and sorted; a file no linked language reads and a
//! rule's fault cross as sentences; the pack of a load is read back by the next run given the same, before it has a plan, and by
//! none given something else; a run that cannot scan is refused; what the engine notices of a rule's work is handed over and
//! worded, and fails the file only where the scan is set to; and a session's workers say what they counted, with a ledger
//! when the run measures.
use super::fixtures::{Edge, composed, contract, file, rule};
use super::interface::FileScanUseCase as _;
use super::types::{RuleSetNumber, ScanLoadRequest, ScanSession, ScanStoredRequest, Scanned};
use super::use_case::FileScanUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::files::types::SourcePath;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::scan_rows::types::{KeptCell, RowPlace, ScanRow};
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::StoredPack;
use std::borrow::Cow;
use std::sync::atomic::Ordering;

/// What a row does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The old engine alone, with a rule the new engine would accept.
    Old,
    /// The new engine alone: three files scanned in one worker.
    New,
    /// Both engines, with a rule the new engine refuses: no file is scanned.
    BothNothingAccepted,
    /// The new engine alone, twice: the second run, given the same, asks the store of the first one's packs before it has a plan.
    SecondRun,
    /// The new engine alone, twice: the second run, given something else, asks the store of the first one's packs.
    SecondRunOfOtherRules,
    /// The new engine, at an edge that composed no engine.
    NoEngine,
    /// The new engine, with a language linked without its walk.
    NoWalk,
    /// The new engine alone, as a user's run, the engine noticing the rule in the second file.
    Overrun,
    /// The same, the scan set to fail on an overrun.
    Gate,
    /// The new engine alone, in a run that measures.
    Measuring,
}

type Input = Scenario;
/// What the use case answered, in lines.
type Value = Cow<'static, str>;
/// Why the run was refused.
type Failure = Cow<'static, str>;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_old_engine_alone_loads_no_rule_set"),
        word: Word::Succeeds,
        input: || Scenario::Old,
        expect: Expect::Value(Cow::Borrowed(
            "Old: rule sets []\na.walk: by None; kinds None; fault Some((NoLanguage, \"file not walked: the language does not read this file\"))\nb.txt: by None; kinds None; fault Some((NoLanguage, \"file not walked: the language does not read this file\"))\nc.walk: by None; kinds None; fault Some((NoLanguage, \"file not walked: the language does not read this file\"))",
        )),
    },
    Case {
        name: CaseName("the_new_engine_scans_each_file_in_the_worker_that_took_it"),
        word: Word::Succeeds,
        input: || Scenario::New,
        expect: Expect::Value(Cow::Borrowed(
            "New: rule sets [[SelectedRuleOrdinal(0)]]\na.walk: by Some(0); kinds Some([Node, File, Span, Symbol]); fault None\n  rule 0 at 0..3: 1 0 3 two\n  rule 0 at 4..7: 2 0 17179869191 one\nb.txt: by None; kinds None; fault Some((NoLanguage, \"file not walked: the language does not read this file\"))\nc.walk: by Some(0); kinds Some([Node, File, Span, Symbol]); fault Some((Rule(SelectedRuleOrdinal(0)), \"rule SelectedRuleOrdinal(0): relation SyntaxError read in part: PartialCause(\\\"a question the fixture cannot read\\\")\"))",
        )),
    },
    Case {
        name: CaseName("both_engines_scan_no_file_of_a_rule_set_that_runs_no_rule"),
        word: Word::Succeeds,
        input: || Scenario::BothNothingAccepted,
        expect: Expect::Value(Cow::Borrowed(
            "Both: rule sets [[]]\na.walk: by None; kinds None; fault Some((NoLanguage, \"file not walked: the language does not read this file\"))\nb.txt: by None; kinds None; fault Some((NoLanguage, \"file not walked: the language does not read this file\"))\nc.walk: by None; kinds None; fault Some((NoLanguage, \"file not walked: the language does not read this file\"))",
        )),
    },
    Case {
        name: CaseName("a_second_run_reads_the_first_ones_pack_with_no_plan_and_lowers_nothing"),
        word: Word::Succeeds,
        input: || Scenario::SecondRun,
        expect: Expect::Value(Cow::Borrowed(
            "New: rule sets [[SelectedRuleOrdinal(0)]]\nagain New: rule sets [[SelectedRuleOrdinal(0)]]; rule 0 emits Some((1, [ColumnIndex(0)], [Heading(\"id\"), Heading(\"file\"), Heading(\"span\"), Heading(\"text\")])); a rule it does not run emits false; the same packs kept: true; lowered since 0; packs read 1",
        )),
    },
    Case {
        name: CaseName("a_second_run_given_something_else_reads_no_pack_back"),
        word: Word::Succeeds,
        input: || Scenario::SecondRunOfOtherRules,
        expect: Expect::Value(Cow::Borrowed(
            "New: rule sets [[SelectedRuleOrdinal(0)]]\nagain no rule set; lowered since 0; packs read 0",
        )),
    },
    Case {
        name: CaseName("an_edge_that_composed_no_engine_is_refused"),
        word: Word::Refuses,
        input: || Scenario::NoEngine,
        expect: Expect::Error(Cow::Borrowed(
            "engine harness: RYO_ENGINE is set and this edge composed no engine port",
        )),
    },
    Case {
        name: CaseName("a_language_linked_without_its_walk_is_refused"),
        word: Word::Refuses,
        input: || Scenario::NoWalk,
        expect: Expect::Error(Cow::Borrowed(
            "engine harness: RYO_ENGINE=new: the language `words` is linked without its walk, so the new engine scans none of its files",
        )),
    },
    Case {
        name: CaseName(
            "a_users_run_hands_an_overrun_over_and_answers_the_file_as_it_would_without_it",
        ),
        word: Word::Succeeds,
        input: || Scenario::Overrun,
        expect: Expect::Value(Cow::Borrowed(
            "New: rule sets [[SelectedRuleOrdinal(0)]]\na.walk: by Some(0); kinds Some([Node, File, Span, Symbol]); fault None\n  rule 0 at 0..3: 1 0 3 two\n  rule 0 at 4..7: 2 0 17179869191 one\nb.walk: by Some(0); kinds Some([Node, File, Span, Symbol]); fault None\n  rule 0 at 0..5: 4294967297 1 5 three\n  rule 0 at 6..10: 4294967298 1 25769803786 four\n  noticed rule 0 step 2: warning: rule `rule` is slow in `b.walk`\n  step 2 of the rule's plan joins its left input (1000 rows) with its right input (1000 rows) with no key between them\n  that is 1000000 pairs in this one file; the run continues and its findings are complete\n  to fix: join on a shared column or add a range, so each row meets only its matches\ncounted [4, 4, 0, 0, 4]; ledger []; counting states 0",
        )),
    },
    Case {
        name: CaseName("a_gates_run_answers_that_the_file_overran_with_every_row_found"),
        word: Word::Succeeds,
        input: || Scenario::Gate,
        expect: Expect::Value(Cow::Borrowed(
            "New: rule sets [[SelectedRuleOrdinal(0)]]\na.walk: by Some(0); kinds Some([Node, File, Span, Symbol]); fault None\n  rule 0 at 0..3: 1 0 3 two\n  rule 0 at 4..7: 2 0 17179869191 one\nb.walk: overran, and the run fails\nb.walk: by Some(0); kinds Some([Node, File, Span, Symbol]); fault None\n  rule 0 at 0..5: 4294967297 1 5 three\n  rule 0 at 6..10: 4294967298 1 25769803786 four\n  noticed rule 0 step 2: warning: rule `rule` is slow in `b.walk`\n  step 2 of the rule's plan joins its left input (1000 rows) with its right input (1000 rows) with no key between them\n  that is 1000000 pairs in this one file; the run continues and its findings are complete\n  to fix: join on a shared column or add a range, so each row meets only its matches",
        )),
    },
    Case {
        name: CaseName("a_measuring_runs_workers_count_and_the_session_has_its_ledger"),
        word: Word::Succeeds,
        input: || Scenario::Measuring,
        expect: Expect::Value(Cow::Borrowed(
            "New: rule sets [[SelectedRuleOrdinal(0)]]\na.walk: by Some(0); kinds Some([Node, File, Span, Symbol]); fault None\n  rule 0 at 0..3: 1 0 3 two\n  rule 0 at 4..7: 2 0 17179869191 one\nb.walk: by Some(0); kinds Some([Node, File, Span, Symbol]); fault None\n  rule 0 at 0..5: 4294967297 1 5 three\n  rule 0 at 6..10: 4294967298 1 25769803786 four\ncounted [4, 4, 0, 0, 4]; ledger [\"unit=Rule:0 class=Navigation rows=4 steps=0 nanos=0 idle_nanos=0 owners=0\"]; counting states 1",
        )),
    },
];

/// A row as a line: its rule, where it is, and its cells.
fn line(row: &ScanRow) -> String {
    let cells: Vec<String> = row
        .cells
        .iter()
        .map(|cell| match cell {
            KeptCell::Text(word) => word.0.to_string(),
            KeptCell::Number(number) => number.0.to_string(),
        })
        .collect();
    let place = match row.place {
        RowPlace::Unplaced => "nowhere".to_owned(),
        RowPlace::At(span) => format!("{}..{}", span.start.0, span.end.0),
    };
    format!("  rule {} at {place}: {}", row.rule.0, cells.join(" "))
}

/// What a session says of itself: its rule sets' accepted rules.
fn said(scan: &FileScanUseCaseImpl, session: &ScanSession) -> String {
    let sets: Vec<String> = (0..session.rule_sets.len())
        .map(|at| format!("{:?}", scan.accepted(session, RuleSetNumber(at))))
        .collect();
    format!("{:?}: rule sets [{}]", scan.mode(), sets.join(", "))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let (scan, fake) = composed(match scenario {
        Scenario::Old => Edge::Old,
        Scenario::New | Scenario::SecondRun | Scenario::SecondRunOfOtherRules => Edge::New,
        Scenario::BothNothingAccepted => Edge::Both,
        Scenario::NoEngine => Edge::NoEngine,
        Scenario::NoWalk => Edge::NoWalk,
        Scenario::Overrun => Edge::Overrun,
        Scenario::Gate => Edge::Gate,
        Scenario::Measuring => Edge::Measuring,
    });
    let plan = match scenario {
        // A column the fake does not find: the rule is refused.
        Scenario::BothNothingAccepted => rule(&["id", "kind"]),
        _ => rule(&["id", "file", "span", "text"]),
    };
    let load = |stored: &[StoredPack]| {
        scan.load(ScanLoadRequest {
            plans: &[&plan],
            bindings: &[],
            statistics: ByteSlice(&[]),
            given: Digest([1; 16]),
            stored,
        })
        .map_err(|error| Cow::Owned(error.to_string()))
    };
    let session = load(&[])?;
    let mut lines = vec![said(&scan, &session)];
    if let Scenario::SecondRun | Scenario::SecondRunOfOtherRules = scenario {
        let kept = scan.packs(&session);
        let before = fake.lowered.load(Ordering::Relaxed);
        let given = match scenario {
            Scenario::SecondRun => Digest([1; 16]),
            _ => Digest([2; 16]),
        };
        // The second run has no plan: it asks the store before it compiles.
        let again = scan
            .stored(ScanStoredRequest {
                given,
                selected: Count(1),
                stored: &kept,
            })
            .map_err(|error| Cow::Owned(error.to_string()))?;
        let read = match &again {
            Some(again) => {
                let emit = scan.emit(again, RuleSetNumber(0), SelectedRuleOrdinal(0));
                format!(
                    "{}; rule 0 emits {:?}; a rule it does not run emits {:?}; the same packs kept: {}",
                    said(&scan, again),
                    emit.map(|emit| (emit.subject.0, &emit.key, &emit.headings)),
                    scan.emit(again, RuleSetNumber(0), SelectedRuleOrdinal(1))
                        .is_some(),
                    scan.packs(again) == kept,
                )
            }
            None => "no rule set".to_owned(),
        };
        lines.push(format!(
            "again {read}; lowered since {}; packs read {}",
            fake.lowered.load(Ordering::Relaxed) - before,
            fake.unpacked.load(Ordering::Relaxed),
        ));
        return Ok(Cow::Owned(lines.join("\n")));
    }
    let mut worker = scan.worker(&session);
    let files: Vec<(&str, &[u8])> = match scenario {
        Scenario::Overrun | Scenario::Gate | Scenario::Measuring => {
            vec![("a.walk", b"two one"), ("b.walk", b"three four")]
        }
        _ => vec![
            ("a.walk", b"two one"),
            ("b.txt", b"two"),
            ("c.walk", b"why?"),
        ],
    };
    let (mut rows, mut noticed) = (Vec::new(), Vec::new());
    for (number, (path, source)) in files.into_iter().enumerate() {
        rows.clear();
        noticed.clear();
        let number = u32::try_from(number).unwrap_or(u32::MAX);
        let scanned = scan.scan(
            &session,
            &mut worker,
            file(number, path, source),
            &mut rows,
            &mut noticed,
        );
        let (by, fault) = match scanned {
            Scanned::Whole(by) => (Some(by), None),
            Scanned::Faulted(by, fault) => (Some(by), Some(fault)),
            Scanned::NotWalked(fault) => (None, Some(fault)),
            Scanned::Overran(by) => {
                lines.push(format!("{path}: overran, and the run fails"));
                (Some(by), None)
            }
        };
        let kinds = by.map(|by| scan.kinds(&session, by, SelectedRuleOrdinal(0)));
        lines.push(format!(
            "{path}: by {:?}; kinds {kinds:?}; fault {:?}",
            by.map(|by| by.0),
            fault.map(|fault| (fault.of, fault.sentence.0)),
        ));
        lines.extend(rows.iter().map(line));
        let id = RuleId("rule".into());
        lines.extend(noticed.iter().map(|each| {
            let warning = scan.worded(each, &id, SourcePath(path), None, None);
            format!(
                "  noticed rule {} step {}: {}",
                each.rule.0, each.step.0, warning.0
            )
        }));
    }
    if let Scenario::Measuring | Scenario::Overrun = scenario {
        let measured = scan.measured(&session, &[&worker]);
        let ledger: Vec<&str> = measured.ledger.iter().map(|line| &*line.0).collect();
        lines.push(format!(
            "counted {:?}; ledger {ledger:?}; counting states {}",
            measured.counted.map(|count| count.0),
            fake.measuring.load(Ordering::Relaxed)
        ));
    }
    Ok(Cow::Owned(lines.join("\n")))
}

#[test]
fn succeeds() {
    for edge in [Edge::Old, Edge::New, Edge::Both] {
        contract::check(&composed(edge).0);
    }
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
