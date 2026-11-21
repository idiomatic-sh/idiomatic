//! The steps of the `scan` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_engine::domain::findings::types::{HeaviestWork, WorkVerdict};
use ryo_engine::inbound::use_cases::scan::fixtures::{
    Scanned, every_row, one_group, rule_rows, scanned,
};

use crate::Behavior;

/// Names the scene the next `scan` step runs.
#[given(expr = "the scan input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Loads the scene's rule set and scans a file of `text` with it through the fixture language.
#[when(expr = "scan runs over {string}")]
fn runs(world: &mut Behavior, text: String) {
    world.found = String::new();
    world.scanned = None;
    world.text.clone_from(&text);
    (world.answer, world.filled) =
        match crate::load::lowered(&world.input).map(|programs| scanned(&programs, &text)) {
            Ok(Ok(Some(scan))) => said(world, scan),
            Ok(walked) => (format!("{walked:?}"), String::new()),
            Err(refused) => (refused.clone(), refused),
        };
}

/// What a scan answered and how many fills it asked, as the steps read them; the scan is kept for the steps that read more.
fn said(world: &mut Behavior, scan: Scanned) -> (String, String) {
    let fills = scan.fills.0.to_string();
    let answer = match &scan.answer {
        Ok(counters) => {
            world.found = scan.found.0.to_string();
            format!(
                "{} candidates, {} steps",
                counters.candidates.0, counters.steps.0
            )
        }
        Err(error) => error.to_string(),
    };
    world.scanned = Some(scan);
    (answer, fills)
}

/// A count a scenario writes.
fn count(written: &str) -> usize {
    let Ok(count) = written.parse() else {
        panic!("`{written}` is not a count");
    };
    count
}

/// Scans a file of that many words of one text: one key group.
#[when(expr = "scan runs over a file of {string} words in one group")]
fn runs_over_a_group(world: &mut Behavior, words: String) {
    world.words = count(&words);
    runs(world, one_group(world.words));
}

/// Scans a file of that many events and that many probes, all of one text: one key group.
#[when(expr = "scan runs over a file of {string} events and {string} probes in one group")]
fn runs_over_events(world: &mut Behavior, events: String, probes: String) {
    world.words = count(&events) + count(&probes);
    runs(world, one_group(world.words));
}

/// Asserts what the scan counted.
#[then(expr = "scan counted {string} candidates and {string} steps")]
fn counted(world: &mut Behavior, candidates: String, steps: String) {
    assert_eq!(
        world.answer,
        format!("{candidates} candidates, {steps} steps")
    );
}

/// Asserts how many relations the language was asked to fill.
#[then(expr = "the language filled {string} relations")]
fn filled(world: &mut Behavior, relations: String) {
    assert_eq!(world.filled, relations);
}

/// Asserts how many rows the scan handed over; a scan that did not run, or faulted, handed over none to count.
#[then(expr = "scan found {string} rows")]
fn found(world: &mut Behavior, rows: String) {
    assert_eq!(world.found, rows, "the scan answered: {}", world.answer);
}

/// Asserts what `scan` answered.
#[then(expr = "scan answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}

/// The scan the last when step kept: a scenario that reads it without one fails here.
fn kept(world: &Behavior) -> &Scanned {
    let Some(scan) = &world.scanned else {
        panic!("the scan did not run: {}", world.answer);
    };
    scan
}

/// The rows the unit a notice names read or gave in the file, by its kind: a product's is each row of one side with each of
/// the other.
const fn worked(heaviest: HeaviestWork) -> u64 {
    match heaviest {
        HeaviestWork::Program { visited, .. } | HeaviestWork::Calls { visited, .. } => visited.0,
        HeaviestWork::Product { left, right } => left.0.saturating_mul(right.0),
        HeaviestWork::Rows { read, made }
        | HeaviestWork::Group { read, made }
        | HeaviestWork::Graph { read, made } => read.0.saturating_add(made.0),
        HeaviestWork::Sorted { rows } => rows.0,
        HeaviestWork::OrderedPairs { left, right, made } => {
            left.0.saturating_add(right.0).saturating_add(made.0)
        }
    }
}

/// How many rules the scan noticed over their allowance. A notice that does not say what did the work fails here: the unit
/// it names as the heaviest must have read or given a row.
fn over(scan: &Scanned) -> usize {
    let noticed = scan
        .notices
        .iter()
        .filter(|notice| notice.verdict == WorkVerdict::Over);
    noticed
        .inspect(|notice| {
            assert_ne!(worked(notice.heaviest), 0, "{notice:?} names no work");
        })
        .count()
}

/// Asserts how many candidates the scan ran and how many times it made a table.
#[then(expr = "scan counted {string} candidates and made the table {string} times")]
fn counted_and_made(world: &mut Behavior, candidates: String, times: String) {
    let scan = kept(world);
    let ran = scan
        .answer
        .as_ref()
        .map(|counters| counters.candidates.0.to_string());
    assert_eq!(
        (ran.ok(), scan.tables_made.0.to_string()),
        (Some(candidates), times),
        "{}",
        world.answer
    );
}

/// Asserts how many times the scan made a table.
#[then(expr = "scan made the table {string} times")]
fn made(world: &mut Behavior, times: String) {
    assert_eq!(
        kept(world).tables_made.0.to_string(),
        times,
        "the scan answered: {}",
        world.answer
    );
}

/// Asserts how many times the scan evaluated a definition for a key.
#[then(expr = "scan evaluated the definition {string} times")]
fn evaluated(world: &mut Behavior, times: String) {
    assert_eq!(
        kept(world).evaluations.0.to_string(),
        times,
        "the scan answered: {}",
        world.answer
    );
}

/// Asserts how many fills the language was asked by one key, and how many whole.
#[then(expr = "the language filled {string} keys and {string} relations whole")]
fn filled_by_key(world: &mut Behavior, keys: String, whole: String) {
    let scan = kept(world);
    let by_key = scan.key_fills.0;
    assert_eq!(
        (by_key.to_string(), (scan.fills.0 - by_key).to_string()),
        (keys, whole),
        "{}",
        world.answer
    );
}

/// Asserts that the scan handed over every row the scene's rule has in the file, and noticed that many rules over their allowance.
#[then(expr = "scan found every row and noticed {string} rule(s) over its allowance")]
fn found_every_row_and_noticed(world: &mut Behavior, rules: String) {
    found_every_row(world, rules);
}

/// Asserts that the scan handed over every row the scene's rule has in the file, and noticed that many rules.
#[then(expr = "scan found every row and noticed {string} rules")]
fn found_every_row(world: &mut Behavior, rules: String) {
    let every = every_row(&world.input, world.words).map(|rows| rows.0.to_string());
    assert_eq!(
        Some(world.found.clone()),
        every,
        "the scan answered: {}",
        world.answer
    );
    noticed(world, rules);
}

/// Asserts how many rules the scan noticed over their allowance.
#[then(expr = "scan noticed {string} rules")]
fn noticed(world: &mut Behavior, rules: String) {
    assert_eq!(
        over(kept(world)).to_string(),
        rules,
        "the scan answered: {}",
        world.answer
    );
}

/// The rows of one rule in the kept scan; a scenario writes the rule's ordinal.
fn rows_of(world: &Behavior, rule: &str) -> usize {
    rule_rows(kept(world), u32::try_from(count(rule)).unwrap_or(u32::MAX))
}

/// Asserts that a rule of the scanned rule set found the rows the scene of that rule alone finds in the same file: a relation
/// read whole after it was read by key gives each row once.
#[then(expr = "rule {string} found as many rows as {string} finds alone, and at least one")]
fn found_as_alone(world: &mut Behavior, rule: String, scene: String) {
    let together = rows_of(world, &rule);
    let alone = crate::load::lowered(&scene).map(|programs| scanned(&programs, &world.text));
    let Ok(Ok(Some(alone))) = alone else {
        panic!("the scene `{scene}` did not scan alone: {alone:?}");
    };
    assert_ne!(together, 0, "the rule found no row");
    assert_eq!(
        together, alone.found.0,
        "the rule found other rows than alone"
    );
}

/// Asserts that one rule found rows and another found more: the first's tree leaves out elements the second's holds.
#[then(expr = "rule {string} found fewer rows than rule {string}, and at least one")]
fn found_fewer(world: &mut Behavior, fewer: String, more: String) {
    let (own, every) = (rows_of(world, &fewer), rows_of(world, &more));
    assert_ne!(own, 0, "the rule found no row");
    assert!(own < every, "{own} rows are not fewer than {every}");
}
