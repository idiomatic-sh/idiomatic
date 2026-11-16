//! The replay of every recorded rule over ruff, run once, and the answer to a question about it: each rule held to its recorded
//! rows, the golden findings, what it could not decide and the features it fed. The `lint` bench's `--replay` run asks it; the
//! comparison of one rule's rows with its recording is `answers`'.

use super::answers::{
    Comparison, Expectation, by_rule, compare, expectation_at, extra_rows_in, golden_count,
    golden_rules, golden_strays, listing, unfed_in, unknowns_in, verdict,
};
use super::base::{Pending, Recording, directory, recordings, required_text, rule_of};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

/// What ryo answered for the rules it was asked: each rule's rows as `path<TAB>key`, and the files nothing read.
pub(crate) struct Replayed {
    pub rows: BTreeMap<String, Vec<String>>,
    pub unread: Vec<String>,
    /// How many things each rule could not decide, by rule.
    pub unknowns: BTreeMap<String, usize>,
    /// How many rows each feature of the language was fed over ruff, by the feature's name.
    pub fed: BTreeMap<String, u64>,
}

/// Runs every rule that can be replayed over ruff, once.
pub(crate) fn replay(recordings: &[Recording]) -> Result<Replayed, String> {
    let rules: Vec<ryo_core::inbound::use_cases::lint::types::Rule> = recordings
        .iter()
        .filter(|each| each.pending == Pending::Missing)
        .map(rule_of)
        .collect();
    let started = std::time::Instant::now();
    super::base::COUNTING.store(true, std::sync::atomic::Ordering::Relaxed);
    let ran = super::base::lint_report(
        &super::snapshot::ruff(),
        rules,
        std::sync::Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        std::sync::Arc::new(
            ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default(),
        ),
    );
    super::base::COUNTING.store(false, std::sync::atomic::Ordering::Relaxed);
    let (rows, report) = ran?;
    let mut unknowns: BTreeMap<String, usize> = BTreeMap::new();
    for each in &report.unknowns {
        *unknowns.entry(each.rule.0.to_string()).or_default() += 1;
    }
    let fed: BTreeMap<String, u64> = report
        .features
        .iter()
        .map(|each| (feature_name(each.feature), each.rows.0))
        .collect();
    let unread: Vec<String> = report
        .unread
        .into_iter()
        .map(|file| file.path.0.to_string())
        .collect();
    eprintln!("the replay took {} ms", started.elapsed().as_millis());
    let live: Vec<&str> = recordings
        .iter()
        .filter(|each| each.pending == Pending::Missing)
        .map(|each| each.name.as_str())
        .collect();
    Ok(Replayed {
        rows: by_rule(&rows, &live)?,
        unread,
        unknowns,
        fed,
    })
}

/// A feature as the listings write it: `operation:join`, `family:glob-any`, `direction:nearest-ancestor`, `relation:syntax-tree`.
pub(crate) fn feature_name(feature: ryo_values::domain::run_metadata::types::Feature) -> String {
    use ryo_values::domain::run_metadata::types::Feature;
    match feature {
        Feature::Operation(kind) => format!("operation:{kind:?}"),
        Feature::Family(test) => format!("family:{test:?}"),
        Feature::Direction(direction) => format!("direction:{direction:?}"),
        Feature::Relation(relation) => format!("relation:{relation:?}"),
    }
}

/// Runs the replay once for the whole test binary: the scenarios that ask for it wait for the one that runs it.
pub(crate) fn replayed(recordings: &[Recording]) -> &'static Result<Replayed, String> {
    static COMPUTED: Mutex<Option<&'static Result<Replayed, String>>> = Mutex::new(None);
    let mut held = COMPUTED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    held.get_or_insert_with(|| Box::leak(Box::new(replay(recordings))))
}

/// What `each` expects, read from its recording.
pub(crate) fn expectation(each: &Recording) -> Expectation {
    expectation_at(&directory().join(&each.name))
}

/// What the replay compares for `each`: its rows `given` against its recording and its recorded differences.
pub(crate) fn comparison(each: &Recording, given: &[String]) -> Comparison {
    compare(
        &each.name,
        &expectation(each),
        &recorded_extra_rows(&each.name),
        given,
    )
}

/// The rows ryo gives for `rule` that the recording does not have, as `path<TAB>key`, from `fixtures/differences.toml`: one table for
/// each rule with `extra` (the rows) and `why` (what ryo sees that the recording could not).
fn recorded_extra_rows(rule: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/differences.toml");
    let table: toml::Table = required_text(&path)
        .parse()
        .expect("the recorded row differences");
    extra_rows_in(&table, rule, &path)
}

/// What the replay answers for `input`.
///
/// # Panics
///
/// When there is nothing recorded to replay.
pub(crate) fn answer(input: &str) -> String {
    let all = recordings();
    assert!(
        !all.is_empty(),
        "nothing is recorded in {}: step 1 of the proof order (README A10) records today's Tastemaker against today's ryo on ruff",
        directory().display()
    );
    let outcome = match replayed(&all) {
        Ok(outcome) => outcome,
        Err(failure) => return failure.clone(),
    };
    let live: Vec<&Recording> = all
        .iter()
        .filter(|each| each.pending == Pending::Missing)
        .collect();
    if live.is_empty() {
        return "no recorded rule is live, so nothing is replayed".to_owned();
    }
    match input {
        "unknowns" => {
            let wrong = unknown_differences(&outcome.unknowns);
            for each in &wrong {
                eprintln!("{each}");
            }
            wrong.first().cloned().unwrap_or_else(|| {
                "every unknown is the golden's or a recorded difference".to_owned()
            })
        }
        "recorded_rules" => recorded_rules(&all, &live, outcome),
        "a_wrong_row" => {
            let Some(first) = live.iter().find(|each| each.name == "deep-nesting") else {
                return "nothing can be replayed".to_owned();
            };
            let mut altered: Vec<String> =
                outcome.rows.get(&first.name).cloned().unwrap_or_default();
            altered.insert(0, "a row nobody recorded".to_owned());
            verdict(&comparison(first, &altered)).unwrap_or_else(|| "no difference".to_owned())
        }
        "recognition" => {
            let recognized = required_text(&directory().join("recognized.txt"));
            let unread: Vec<&str> = recognized
                .lines()
                .filter(|path| outcome.unread.iter().any(|each| each == path))
                .collect();
            match unread.first() {
                None => "the unread files are the recorded ones".to_owned(),
                Some(first) => format!("{} recorded files are unread: {first}", unread.len()),
            }
        }
        "findings" => findings(&live, outcome),
        "features" => features(outcome),
        "names" => names(&all, outcome),
        other => panic!("the replay has no input `{other}`"),
    }
}

/// Every live rule against its recorded rows, each rule's verdict said on stderr, and every row of a rule that fails listed there
/// with its source text: the first failure, or that every rule gives its rows.
fn recorded_rules(all: &[Recording], live: &[&Recording], outcome: &Replayed) -> String {
    let none = Vec::new();
    let ruff = super::snapshot::ruff();
    let verdicts: BTreeMap<&str, Option<String>> = live
        .iter()
        .map(|each| {
            let compared = comparison(each, outcome.rows.get(&each.name).unwrap_or(&none));
            let failed = verdict(&compared);
            if failed.is_some() {
                for line in listing(&compared, &ruff) {
                    eprintln!("{line}");
                }
            }
            (each.name.as_str(), failed)
        })
        .collect();
    let wrong: Vec<&String> = verdicts.values().flatten().collect();
    for each in all {
        let said = expectation(each);
        let status = match (
            &each.pending,
            said.undecided,
            verdicts.get(each.name.as_str()).and_then(Option::as_ref),
        ) {
            (Pending::Because(why), _, _) => format!("pending: {why}"),
            (Pending::Missing, true, _) => format!(
                "checked by hand ({} rows; today's ryo could not decide it; {} findings of ruff checked against the source)",
                outcome.rows.get(&each.name).map_or(0, Vec::len),
                said.checked.len()
            ),
            (Pending::Missing, false, Some(first)) => format!("DIFFERS: {first}"),
            (Pending::Missing, false, None) => {
                format!("matches ({} rows)", said.expected.len())
            }
        };
        eprintln!("{:40} {status}", each.name);
    }
    eprintln!(
        "{} of {} replayed rules match or run; {} are pending",
        live.len().saturating_sub(wrong.len()),
        live.len(),
        all.len().saturating_sub(live.len())
    );
    wrong
        .first()
        .map(|first| (*first).clone())
        .unwrap_or_else(|| "every rule gives its rows".to_owned())
}

/// The golden findings of ruff: `findings.toml`, which must be there and hold a `rules` table.
fn golden() -> toml::Table {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/replay/targets/ruff/findings.toml");
    let table: toml::Table = required_text(&path)
        .parse()
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    golden_rules(&table, &path).clone()
}

/// The distinct rows of each live rule against the count `findings.toml` expects of ruff: the first that differs, or that they match.
fn findings(live: &[&Recording], outcome: &Replayed) -> String {
    let rules = golden();
    let wrong: Vec<String> = live
        .iter()
        .filter_map(|each| {
            let wanted = match golden_count(&rules, &each.name, expectation(each).undecided) {
                Ok(Some(wanted)) => wanted,
                Ok(None) => return None,
                Err(failure) => return Some(failure),
            };
            // Ryo's distinct rows, without the ones `fixtures/differences.toml` says ryo gives and the golden could not.
            let got = comparison(
                each,
                outcome.rows.get(&each.name).map_or(&[], Vec::as_slice),
            )
            .given
            .len();
            (got != wanted).then(|| format!("{} gives {got}, ruff expects {wanted}", each.name))
        })
        .collect();
    wrong
        .first()
        .cloned()
        .unwrap_or_else(|| "the findings are the expected ones".to_owned())
}

/// Whether each feature of the language is fed enough rows over ruff, or `fixtures/unfed.toml` says why not.
fn features(outcome: &Replayed) -> String {
    // Each feature of the language is fed at least this many rows over ruff, unless `fixtures/unfed.toml` says why not.
    const FED: u64 = 1_000;
    let mut fed = outcome.fed.clone();
    #[cfg(feature = "python")]
    match super::feature_contracts::measured() {
        Ok(measured) => {
            for (name, rows) in measured {
                *fed.entry(name.clone()).or_default() += rows;
            }
        }
        Err(failure) => return failure.clone(),
    }
    let unfed = recorded_unfed();
    let short: Vec<String> = fed
        .iter()
        .filter(|(name, rows)| **rows < FED && !unfed.contains(name))
        .map(|(name, rows)| format!("{name} ({rows} rows)"))
        .collect();
    for each in &short {
        eprintln!("unfed: {each}");
    }
    let gone: Vec<&String> = unfed
        .iter()
        .filter(|name| fed.get(*name).is_none_or(|rows| *rows >= FED))
        .collect();
    match (short.first(), gone.first()) {
        (Some(first), _) => format!(
            "{} features are fed fewer than {FED} rows over ruff and are not in fixtures/unfed.toml, the first {first}",
            short.len()
        ),
        (None, Some(name)) => format!(
            "fixtures/unfed.toml lists `{name}` and it is fed {FED} rows or more now, or is not a feature"
        ),
        (None, None) => "every feature is fed".to_owned(),
    }
}

/// Whether every name in the files that say why something differs is a recorded rule or a feature of the language.
fn names(all: &[Recording], outcome: &Replayed) -> String {
    // Every name in the files that say why something differs, and every rule of the golden findings, is a rule that is recorded or a
    // feature the language has, so a typo, a deleted or a renamed recording fails at once and not when it happens to matter.
    let table = |file: &str, key: &str| -> Vec<String> {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures")
            .join(file);
        let table: toml::Table = required_text(&path).parse().expect("a table that reads");
        table
            .get(key)
            .and_then(toml::Value::as_table)
            .into_iter()
            .flat_map(|named| named.keys().cloned())
            .collect()
    };
    let rules: Vec<&str> = all.iter().map(|each| each.name.as_str()).collect();
    let unknown = ["differences.toml", "unknowns.toml"]
        .iter()
        .flat_map(|file| {
            table(file, "rule")
                .into_iter()
                .filter(|name| !rules.contains(&name.as_str()))
                .map(move |name| format!("`{name}` in {file} is not a recorded rule"))
        })
        .chain(golden_strays(&golden(), &rules))
        .chain(
            table("unfed.toml", "feature")
                .into_iter()
                .filter(|name| !outcome.fed.contains_key(name))
                .map(|name| format!("`{name}` in unfed.toml is not a feature of the language")),
        )
        .next();
    unknown.unwrap_or_else(|| "every name is a recorded rule or a feature".to_owned())
}

/// The features `fixtures/unfed.toml` says ruff does not feed 1,000 rows, each with the reason.
fn recorded_unfed() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/unfed.toml");
    let table: toml::Table = required_text(&path)
        .parse()
        .expect("the recorded unfed features");
    unfed_in(&table, &path)
}

/// How many things the golden says each rule could not decide over ruff, by rule: the `#unknown` lines of its findings.
fn golden_unknowns() -> BTreeMap<String, usize> {
    unknowns_in(&golden())
}

/// Where ryo and the golden differ in what a rule could not decide, and the rows of `fixtures/unknowns.toml` that say why: each
/// rule that differs has a row with both numbers and a reason, and no row stands for a rule that does not differ.
fn unknown_differences(ours: &BTreeMap<String, usize>) -> Vec<String> {
    let golden = golden_unknowns();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/unknowns.toml");
    let table: toml::Table = required_text(&path)
        .parse()
        .expect("the recorded differences");
    let rows = table.get("rule").and_then(toml::Value::as_table);
    let mut wrong = Vec::new();
    // A row of `fixtures/unknowns.toml` for a rule neither side has an unknown for is a difference that went away, and fails too.
    let recorded: Vec<String> = rows
        .map(|rows| rows.keys().cloned().collect())
        .unwrap_or_default();
    let rules: std::collections::BTreeSet<&String> = ours
        .keys()
        .chain(golden.keys())
        .chain(recorded.iter())
        .collect();
    for rule in rules {
        let (mine, theirs) = (
            ours.get(rule).copied().unwrap_or(0),
            golden.get(rule).copied().unwrap_or(0),
        );
        let row = rows
            .and_then(|rows| rows.get(rule))
            .and_then(toml::Value::as_table);
        let number = |key: &str| {
            let value = row
                .and_then(|row| row.get(key))
                .and_then(toml::Value::as_integer)?;
            usize::try_from(value).ok()
        };
        let why = row
            .and_then(|row| row.get("why"))
            .and_then(toml::Value::as_str)
            .unwrap_or_default();
        match (mine == theirs, row.is_some()) {
            (true, false) => {}
            (true, true) => wrong.push(format!("`{rule}` could not decide as many as the golden's {theirs}, and a recorded difference says it does not")),
            (false, false) => wrong.push(format!("`{rule}` could not decide {mine}, the golden {theirs}, and no difference in `fixtures/unknowns.toml` says why")),
            (false, true) => {
                if number("ryo") != Some(mine) || number("golden") != Some(theirs) || why.trim().is_empty() {
                    wrong.push(format!("`{rule}` could not decide {mine}, the golden {theirs}; its row in `fixtures/unknowns.toml` says {:?} and {:?} with a reason of {} characters", number("ryo"), number("golden"), why.len()));
                }
            }
        }
    }
    wrong
}
