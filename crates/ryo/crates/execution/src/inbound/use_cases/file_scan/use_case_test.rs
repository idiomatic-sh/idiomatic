//! The file scan use case's cases, over the engine port's fake and the fixture language: the old engine alone loads nothing; the
//! new engine's rule set scans a file in a worker and hands its rows, placed and sorted; a file no linked language reads and a
//! rule's fault cross as sentences; the pack of a load is read back by the next; and a run that cannot scan is refused.
use super::fixtures::{Edge, composed, contract, file, rule};
use super::interface::FileScanUseCase as _;
use super::types::{RuleSetNumber, ScanLoadRequest, ScanSession, Scanned};
use super::use_case::FileScanUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
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
    /// The new engine alone, twice: the second load is handed the first one's packs.
    SecondRun,
    /// The new engine, at an edge that composed no engine.
    NoEngine,
    /// The new engine, with a language linked without its walk.
    NoWalk,
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
        name: CaseName("a_second_run_reads_the_first_ones_pack_and_lowers_nothing"),
        word: Word::Succeeds,
        input: || Scenario::SecondRun,
        expect: Expect::Value(Cow::Borrowed(
            "New: rule sets [[SelectedRuleOrdinal(0)]]\nagain New: rule sets [[SelectedRuleOrdinal(0)]]; lowered since 0; packs read 1; the same packs kept: true",
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
        Scenario::New | Scenario::SecondRun => Edge::New,
        Scenario::BothNothingAccepted => Edge::Both,
        Scenario::NoEngine => Edge::NoEngine,
        Scenario::NoWalk => Edge::NoWalk,
    });
    let plan = match scenario {
        // A column the fake does not find: the rule is refused.
        Scenario::BothNothingAccepted => rule(&["id", "kind"]),
        _ => rule(&["id", "file", "span", "text"]),
    };
    let digests = [Digest([1; 16])];
    let load = |stored: &[StoredPack]| {
        scan.load(ScanLoadRequest {
            plans: &[&plan],
            digests: &digests,
            stored,
        })
        .map_err(|error| Cow::Owned(error.to_string()))
    };
    let session = load(&[])?;
    let mut lines = vec![said(&scan, &session)];
    if matches!(scenario, Scenario::SecondRun) {
        let kept = scan.packs(&session);
        let before = fake.lowered.load(Ordering::Relaxed);
        let again = load(&kept)?;
        lines.push(format!(
            "again {}; lowered since {}; packs read {}; the same packs kept: {}",
            said(&scan, &again),
            fake.lowered.load(Ordering::Relaxed) - before,
            fake.unpacked.load(Ordering::Relaxed),
            scan.packs(&again) == kept,
        ));
        return Ok(Cow::Owned(lines.join("\n")));
    }
    let mut worker = scan.worker(&session);
    let files: [(&str, &[u8]); 3] = [
        ("a.walk", b"two one"),
        ("b.txt", b"two"),
        ("c.walk", b"why?"),
    ];
    let mut rows = Vec::new();
    for (number, (path, source)) in files.into_iter().enumerate() {
        rows.clear();
        let number = u32::try_from(number).unwrap_or(u32::MAX);
        let scanned = scan.scan(&session, &mut worker, file(number, path, source), &mut rows);
        let (by, fault) = match scanned {
            Scanned::Whole(by) => (Some(by), None),
            Scanned::Faulted(by, fault) => (Some(by), Some(fault)),
            Scanned::NotWalked(fault) => (None, Some(fault)),
        };
        let kinds = by.map(|by| scan.kinds(&session, by, SelectedRuleOrdinal(0)));
        lines.push(format!(
            "{path}: by {:?}; kinds {kinds:?}; fault {:?}",
            by.map(|by| by.0),
            fault.map(|fault| (fault.of, fault.sentence.0)),
        ));
        lines.extend(rows.iter().map(line));
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
