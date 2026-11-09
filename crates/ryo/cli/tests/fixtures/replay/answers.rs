//! What the replay adds to the base: each recording run again and held to the rows it expects, and the answer to a question about them.

use super::base::{Pending, Recording, directory, recordings, required_text, rule_of, text};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::Mutex;

/// What ryo answered for the rules it was asked: each rule's rows as `path<TAB>key`, and the files nothing read.
/// What the recording of a rule expects.
pub(crate) struct Expectation {
    pub plan: String,
    pub expected: Vec<String>,
    /// Whether today's ryo could not decide the rule, so that nothing was recorded to compare with.
    pub undecided: bool,
    /// The findings of ruff checked by hand against the source (`checked.toml`): a rule with none is checked to give none.
    pub checked: Vec<String>,
}

/// What `each` expects, read from its recording.
pub(crate) fn expectation(each: &Recording) -> Expectation {
    expectation_at(&directory().join(&each.name))
}

/// The rows of the recorded file `file` under `at`, each as `path<TAB>key` the way the replay writes what it finds. The file must
/// be there, and each row must have a path and a key.
pub(crate) fn recorded_rows(at: &Path, file: &str) -> Vec<String> {
    let path = at.join(file);
    let read = std::fs::read_to_string(&path).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let table: toml::Table = read
        .parse()
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    let rows = table
        .get("rows")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{}: no `rows` list", path.display()));
    rows.iter().map(|row| row_text(row, &path)).collect()
}

/// One recorded row, a table with a `path` and a `key`, as `path<TAB>key`.
fn row_text(row: &toml::Value, file: &Path) -> String {
    let field = |name: &str| {
        row.as_table()
            .and_then(|row| row.get(name))
            .and_then(toml::Value::as_str)
            .unwrap_or_else(|| panic!("{}: a row has no `{name}`: {row}", file.display()))
            .to_owned()
    };
    format!("{}\t{}", field("path"), field("key"))
}

/// What the recording in the directory `at` expects. A recording holds its rows in `expected.toml`; one today's ryo could not
/// decide says why in `undecided.txt` and holds the findings checked by hand in `checked.toml`, `rows = []` when there are none.
pub(crate) fn expectation_at(at: &Path) -> Expectation {
    let undecided = at.join("undecided.txt").exists();
    if undecided {
        assert!(
            !text(&at.join("undecided.txt")).trim().is_empty(),
            "{}: undecided.txt names no reason",
            at.display()
        );
    }
    let expected = recorded_rows(at, "expected.toml");
    assert!(
        !undecided || expected.is_empty(),
        "{}: an undecided recording records {} rows that nothing compares",
        at.display(),
        expected.len()
    );
    assert!(
        undecided || !at.join("checked.toml").exists(),
        "{}: checked.toml beside no undecided.txt is compared with nothing",
        at.display()
    );
    Expectation {
        plan: required_text(&at.join("plan")).trim().to_owned(),
        expected,
        undecided,
        checked: if undecided {
            recorded_rows(at, "checked.toml")
        } else {
            Vec::new()
        },
    }
}

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
        &super::base::snapshot::ruff(),
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

/// The findings `rows` by the rule that gave them, each as `path<TAB>key`, sorted. A row of fewer than three cells, or one under a
/// rule that is not in `live`, is a failure: it would otherwise be dropped unseen.
pub(crate) fn by_rule(rows: &[Vec<String>], live: &[&str]) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut by_rule: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        let [rule, path, key, ..] = row.as_slice() else {
            return Err(format!("a finding has fewer than three cells: {row:?}"));
        };
        if !live.contains(&rule.as_str()) {
            return Err(format!("a finding is given under `{rule}`, which is not a live recorded rule: {row:?}"));
        }
        by_rule
            .entry(rule.clone())
            .or_default()
            .push(format!("{path}\t{key}"));
    }
    for each in by_rule.values_mut() {
        each.sort();
    }
    Ok(by_rule)
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

/// How a rule's rows differ from the recorded ones: nothing when they are the same, otherwise the rule, its plan and the first row that differs.
pub(crate) fn difference(each: &Recording, given: &[String]) -> Option<String> {
    let said = expectation(each);
    // A finding is its path and key, once, as it is to the one that reads it.
    let mut given = given.to_vec();
    given.dedup();
    // The rows ryo gives that the recording could not, each with its reason in `fixtures/differences.toml`: a row listed there
    // that is no longer given is a difference that went away, and fails.
    let allowed = recorded_extra_rows(&each.name);
    if let Some(gone) = allowed.iter().find(|row| !given.contains(row)) {
        return Some(format!(
            "rule `{}` has the recorded difference `{gone}` in fixtures/differences.toml and no longer gives it",
            each.name
        ));
    }
    given.retain(|row| !allowed.contains(row));
    let given = given.as_slice();
    let mut wanted = said.expected.clone();
    wanted.dedup();
    let wanted = &wanted;
    let at = wanted
        .iter()
        .zip(given)
        .position(|(left, right)| left != right)
        .or_else(|| (wanted.len() != given.len()).then_some(wanted.len().min(given.len())))?;
    let show = |rows: &[String]| {
        rows.get(at)
            .cloned()
            .unwrap_or_else(|| format!("(no row; {} rows)", rows.len()))
    };
    let missing = wanted
        .iter()
        .filter(|row| given.binary_search(row).is_err())
        .count();
    let extra = given
        .iter()
        .filter(|row| wanted.binary_search(row).is_err())
        .count();
    Some(format!(
        "rule `{name}` (plan `{plan}`) differs at row {row}: recorded `{recorded}`, ryo `{got}` ({missing} recorded rows missing, {extra} rows not recorded; {} recorded, {} ryo)",
        wanted.len(),
        given.len(),
        name = each.name,
        plan = said.plan,
        row = at + 1,
        recorded = show(wanted),
        got = show(given)
    ))
}

/// The rows ryo gives for `rule` that the recording does not have, as `path<TAB>key`, from `fixtures/differences.toml`: one table for
/// each rule with `extra` (the rows) and `why` (what ryo sees that the recording could not).
fn recorded_extra_rows(rule: &str) -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/differences.toml");
    let table: toml::Table = required_text(&path).parse().expect("the recorded row differences");
    extra_rows_in(&table, rule, &path)
}

/// The rows `table`, read from `path`, lists for `rule`. An entry must say why ryo gives them, in a non-empty `why`, and each row
/// must have a path and a key.
pub(crate) fn extra_rows_in(table: &toml::Table, rule: &str, path: &Path) -> Vec<String> {
    let Some(each) = table
        .get("rule")
        .and_then(toml::Value::as_table)
        .and_then(|rules| rules.get(rule))
    else {
        return Vec::new();
    };
    let each = each
        .as_table()
        .unwrap_or_else(|| panic!("{}: `rule.{rule}` is not a table", path.display()));
    assert!(
        each.get("why")
            .and_then(toml::Value::as_str)
            .is_some_and(|why| !why.trim().is_empty()),
        "{}: `rule.{rule}` says no `why`",
        path.display()
    );
    each.get("extra")
        .and_then(toml::Value::as_array)
        .unwrap_or_else(|| panic!("{}: `rule.{rule}` lists no `extra` rows", path.display()))
        .iter()
        .map(|row| row_text(row, path))
        .collect()
}

/// How the rows of a rule today's ryo could not decide differ from what was checked by hand against ruff: the rows it gives,
/// without the ones `fixtures/differences.toml` lists for it with a reason, are exactly the checked findings.
pub(crate) fn unchecked(each: &Recording, given: &[String]) -> Option<String> {
    let said = expectation(each);
    unchecked_rows(&each.name, &said.checked, &recorded_extra_rows(&each.name), given)
}

/// How `given`, the rows of the undecided rule `rule`, differ from the findings `checked` by hand: a checked finding not given, or
/// a row given that is neither checked nor among the rows `allowed` with a reason; a row `allowed` that is no longer given fails too.
pub(crate) fn unchecked_rows(rule: &str, checked: &[String], allowed: &[String], given: &[String]) -> Option<String> {
    if let Some(gone) = allowed.iter().find(|row| !given.contains(row)) {
        return Some(format!(
            "rule `{rule}` has the recorded difference `{gone}` in fixtures/differences.toml and no longer gives it"
        ));
    }
    if let Some(missing) = checked.iter().find(|row| !given.contains(row)) {
        return Some(format!(
            "rule `{rule}` does not give `{missing}`, which was checked by hand against ruff"
        ));
    }
    given
        .iter()
        .find(|row| !checked.contains(row) && !allowed.contains(row))
        .map(|extra| {
            format!(
                "rule `{rule}` gives `{extra}`, which is not among the {} findings checked by hand against ruff nor a recorded difference",
                checked.len()
            )
        })
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
            difference(first, &altered).unwrap_or_else(|| "no difference".to_owned())
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

/// Every live rule against its recorded rows, each rule's verdict said on stderr: the first difference, or that every rule gives its rows.
fn recorded_rules(all: &[Recording], live: &[&Recording], outcome: &Replayed) -> String {
    let none = Vec::new();
    let rows_of = |each: &Recording| outcome.rows.get(&each.name).unwrap_or(&none).clone();
    let wrong: Vec<String> = live
        .iter()
        .filter_map(|each| {
            if expectation(each).undecided {
                unchecked(each, &rows_of(each))
            } else {
                difference(each, &rows_of(each))
            }
        })
        .collect();
    for each in all {
        let said = expectation(each);
        let verdict = match (
            &each.pending,
            said.undecided,
            difference(each, outcome.rows.get(&each.name).unwrap_or(&none)),
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
        eprintln!("{:40} {verdict}", each.name);
    }
    eprintln!(
        "{} of {} replayed rules match or run; {} are pending",
        live.len() - wrong.len(),
        live.len(),
        all.len() - live.len()
    );
    wrong
        .first()
        .cloned()
        .unwrap_or_else(|| "every rule gives its rows".to_owned())
}

/// The golden findings of ruff: `findings.toml`, which must be there and hold a `rules` table.
fn golden() -> toml::Table {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/replay/targets/ruff/findings.toml");
    let table: toml::Table = required_text(&path)
        .parse()
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
    golden_rules(&table, &path).clone()
}

/// The `rules` table of the golden findings `table`, read from `path`: a golden without one fails.
pub(crate) fn golden_rules<'t>(table: &'t toml::Table, path: &Path) -> &'t toml::Table {
    table
        .get("rules")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("{}: no `rules` table", path.display()))
}

/// How many findings the golden `rules` expects of the live rule `rule`: a decided rule has its count, an undecided one has the
/// count of what the golden could not decide (`#unknown`) and no count to compare (`None`). A rule with no entry, or with a count
/// that is not a whole number of zero or more, fails.
pub(crate) fn golden_count(rules: &toml::Table, rule: &str, undecided: bool) -> Result<Option<usize>, String> {
    let key = if undecided {
        format!("idiomatic.arch/{rule}#unknown")
    } else {
        format!("idiomatic.arch/{rule}")
    };
    let count = rules
        .get(&key)
        .ok_or_else(|| format!("the golden findings have no `{key}`"))?
        .as_integer()
        .and_then(|count| usize::try_from(count).ok())
        .ok_or_else(|| format!("the golden count of `{key}` is not a whole number of zero or more"))?;
    Ok((!undecided).then_some(count))
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
            let mut distinct = outcome.rows.get(&each.name).cloned().unwrap_or_default();
            distinct.dedup();
            // Without the rows `fixtures/differences.toml` says ryo gives and the golden could not.
            let extra = recorded_extra_rows(&each.name);
            distinct.retain(|row| !extra.contains(row));
            let got = distinct.len();
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

/// The keys of the golden `rules` that name no recorded rule, each as the failure it is: a key is `idiomatic.arch/<rule>` or
/// `idiomatic.arch/<rule>#unknown`, for a rule that is recorded, so a recording deleted or renamed fails the replay.
pub(crate) fn golden_strays(rules: &toml::Table, recorded: &[&str]) -> Vec<String> {
    rules
        .keys()
        .filter(|key| {
            !key.strip_prefix("idiomatic.arch/")
                .map(|rule| rule.strip_suffix("#unknown").unwrap_or(rule))
                .is_some_and(|rule| recorded.contains(&rule))
        })
        .map(|key| format!("`{key}` in replay/targets/ruff/findings.toml is not a recorded rule"))
        .collect()
}

/// The features `fixtures/unfed.toml` says ruff does not feed 1,000 rows, each with the reason.
fn recorded_unfed() -> Vec<String> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/unfed.toml");
    let table: toml::Table = required_text(&path).parse().expect("the recorded unfed features");
    unfed_in(&table, &path)
}

/// The features `table`, read from `path`, lists as unfed: each must say why, in a non-empty `why`.
pub(crate) fn unfed_in(table: &toml::Table, path: &Path) -> Vec<String> {
    let features = table
        .get("feature")
        .and_then(toml::Value::as_table)
        .unwrap_or_else(|| panic!("{}: no `feature` table", path.display()));
    for (name, each) in features {
        assert!(
            each.get("why")
                .and_then(toml::Value::as_str)
                .is_some_and(|why| !why.trim().is_empty()),
            "{}: `feature.{name}` says no `why`",
            path.display()
        );
    }
    features.keys().cloned().collect()
}

/// How many things the golden says each rule could not decide over ruff, by rule: the `#unknown` lines of its findings.
fn golden_unknowns() -> BTreeMap<String, usize> {
    unknowns_in(&golden())
}

/// The `#unknown` counts of the golden `rules`, by rule. An `#unknown` key not of `idiomatic.arch/<rule>`, or a count that is not
/// a whole number of zero or more, fails.
pub(crate) fn unknowns_in(rules: &toml::Table) -> BTreeMap<String, usize> {
    rules
        .iter()
        .filter_map(|(key, count)| Some((key.strip_suffix("#unknown")?, key, count)))
        .map(|(rule, key, count)| {
            let rule = rule
                .strip_prefix("idiomatic.arch/")
                .unwrap_or_else(|| panic!("the golden key `{key}` names no `idiomatic.arch/` rule"));
            let count = count
                .as_integer()
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or_else(|| panic!("the golden count of `{key}` is not a whole number of zero or more"));
            (rule.to_owned(), count)
        })
        .collect()
}

/// Where ryo and the golden differ in what a rule could not decide, and the rows of `fixtures/unknowns.toml` that say why: each
/// rule that differs has a row with both numbers and a reason, and no row stands for a rule that does not differ.
fn unknown_differences(ours: &BTreeMap<String, usize>) -> Vec<String> {
    let golden = golden_unknowns();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/unknowns.toml");
    let table: toml::Table = required_text(&path).parse().expect("the recorded differences");
    let rows = table.get("rule").and_then(toml::Value::as_table);
    let mut wrong = Vec::new();
    // A row of `fixtures/unknowns.toml` for a rule neither side has an unknown for is a difference that went away, and fails too.
    let recorded: Vec<String> = rows.map(|rows| rows.keys().cloned().collect()).unwrap_or_default();
    let rules: std::collections::BTreeSet<&String> = ours.keys().chain(golden.keys()).chain(recorded.iter()).collect();
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
