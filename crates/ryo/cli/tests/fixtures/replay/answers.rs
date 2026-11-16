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
    let read = std::fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("{}: {error}", path.display()));
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
        fed,
    })
}

/// The findings `rows` by the rule that gave them, each as `path<TAB>key`, sorted. A row of fewer than three cells, or one under a
/// rule that is not in `live`, is a failure: it would otherwise be dropped unseen.
pub(crate) fn by_rule(
    rows: &[Vec<String>],
    live: &[&str],
) -> Result<BTreeMap<String, Vec<String>>, String> {
    let mut by_rule: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for row in rows {
        let [rule, path, key, ..] = row.as_slice() else {
            return Err(format!("a finding has fewer than three cells: {row:?}"));
        };
        if !live.contains(&rule.as_str()) {
            return Err(format!(
                "a finding is given under `{rule}`, which is not a live recorded rule: {row:?}"
            ));
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

/// What a rule's rows are compared against: `expected.toml` for a decided rule, `checked.toml` for one today's ryo could not decide.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Against {
    /// The rows recorded in `expected.toml`.
    Expected,
    /// The findings of ruff checked by hand against the source, in `checked.toml`.
    Checked,
}

/// One rule's replayed rows against its recording: the only comparison, which the verdict, the listing and the findings count read.
/// Every list is `path<TAB>key`, sorted and deduplicated: a finding is its path and key, once, as it is to the one that reads it.
pub(crate) struct Comparison {
    pub rule: String,
    pub plan: String,
    pub against: Against,
    /// `expected.toml` (Expected) or `checked.toml` (Checked), deduplicated.
    pub recorded: Vec<String>,
    /// Ryo's rows, deduplicated, without the recorded differences.
    pub given: Vec<String>,
    /// Recorded rows ryo does not give.
    pub missing: Vec<String>,
    /// Rows ryo gives that are neither recorded nor a recorded difference.
    pub extra: Vec<String>,
    /// Recorded differences (`fixtures/differences.toml`) ryo no longer gives.
    pub gone: Vec<String>,
}

/// `rows` sorted, each once.
fn distinct(rows: &[String]) -> Vec<String> {
    let mut rows = rows.to_vec();
    rows.sort();
    rows.dedup();
    rows
}

/// The rows of `rows` that `other`, sorted, does not hold.
fn without(rows: &[String], other: &[String]) -> Vec<String> {
    rows.iter()
        .filter(|row| other.binary_search(row).is_err())
        .cloned()
        .collect()
}

/// Compares `given` (ryo's rows for `rule`, in any order, duplicates allowed) with what `said` records, taking out `allowed`, the
/// rule's rows in `fixtures/differences.toml`.
pub(crate) fn compare(
    rule: &str,
    said: &Expectation,
    allowed: &[String],
    given: &[String],
) -> Comparison {
    let (against, recorded) = if said.undecided {
        (Against::Checked, distinct(&said.checked))
    } else {
        (Against::Expected, distinct(&said.expected))
    };
    let allowed = distinct(allowed);
    let every = distinct(given);
    let gone = without(&allowed, &every);
    let given = without(&every, &allowed);
    Comparison {
        rule: rule.to_owned(),
        plan: said.plan.clone(),
        against,
        missing: without(&recorded, &given),
        extra: without(&given, &recorded),
        recorded,
        given,
        gone,
    }
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

/// The first failure of `c`, nothing when it has none: a recorded difference no longer given; else, for a decided rule, the rule,
/// its plan and the first row that differs; for an undecided one, a checked finding not given or a row given that nobody checked.
pub(crate) fn verdict(c: &Comparison) -> Option<String> {
    if let Some(gone) = c.gone.first() {
        return Some(format!(
            "rule `{}` has the recorded difference `{gone}` in fixtures/differences.toml and no longer gives it",
            c.rule
        ));
    }
    if c.missing.is_empty() && c.extra.is_empty() {
        return None;
    }
    match c.against {
        Against::Expected => {
            let (wanted, given) = (&c.recorded, &c.given);
            let at = wanted
                .iter()
                .zip(given)
                .position(|(left, right)| left != right)
                .unwrap_or_else(|| wanted.len().min(given.len()));
            let show = |rows: &[String]| {
                rows.get(at)
                    .cloned()
                    .unwrap_or_else(|| format!("(no row; {} rows)", rows.len()))
            };
            Some(format!(
                "rule `{name}` (plan `{plan}`) differs at row {row}: recorded `{recorded}`, ryo `{got}` ({missing} recorded rows missing, {extra} rows not recorded; {} recorded, {} ryo)",
                wanted.len(),
                given.len(),
                name = c.rule,
                plan = c.plan,
                row = at.saturating_add(1),
                recorded = show(wanted),
                got = show(given),
                missing = c.missing.len(),
                extra = c.extra.len(),
            ))
        }
        Against::Checked => match (c.missing.first(), c.extra.first()) {
            (Some(missing), _) => Some(format!(
                "rule `{}` does not give `{missing}`, which was checked by hand against ruff",
                c.rule
            )),
            (None, Some(extra)) => Some(format!(
                "rule `{}` gives `{extra}`, which is not among the {} findings checked by hand against ruff nor a recorded difference",
                c.rule,
                c.recorded.len()
            )),
            (None, None) => None,
        },
    }
}

/// Every row of `c` that differs, one line each: the gone, then the missing, then the extra rows. Each line is five fields
/// separated by tabs: the rule, `gone`, `missing` or `extra`, the path, the key, and the source text at the key under `root`.
pub(crate) fn listing(c: &Comparison, root: &Path) -> Vec<String> {
    [
        ("gone", &c.gone),
        ("missing", &c.missing),
        ("extra", &c.extra),
    ]
    .into_iter()
    .flat_map(|(kind, rows)| {
        rows.iter()
            .map(move |row| format!("{}\t{kind}\t{row}\t{}", c.rule, source_text(root, row)))
    })
    .collect()
}

/// How many characters of a row's source text a listing shows.
const SHOWN: usize = 160;

/// The text at a row's key in `root/path`, the row being `path<TAB>key`: the bytes `start..end` when the key is a span inside
/// the file, read lossily, its first line with tabs as spaces and at most 160 characters. `-` when the key is not a span (a key
/// that is a path, as orphan-rust-files gives); `(outside the file)` when the span ends past the file or the file is absent.
pub(crate) fn source_text(root: &Path, row: &str) -> String {
    let (path, key) = row.split_once('\t').unwrap_or((row, ""));
    let Some((start, end)) = key
        .split_once("..")
        .and_then(|(start, end)| Some((start.parse::<usize>().ok()?, end.parse::<usize>().ok()?)))
    else {
        return "-".to_owned();
    };
    let outside = || "(outside the file)".to_owned();
    let Ok(bytes) = std::fs::read(root.join(path)) else {
        return outside();
    };
    let Some(span) = bytes.get(start..end) else {
        return outside();
    };
    String::from_utf8_lossy(span)
        .lines()
        .next()
        .unwrap_or_default()
        .replace('\t', " ")
        .chars()
        .take(SHOWN)
        .collect()
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
        "names" => names(&all, outcome),
        other => panic!("the replay has no input `{other}`"),
    }
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
pub(crate) fn golden_count(
    rules: &toml::Table,
    rule: &str,
    undecided: bool,
) -> Result<Option<usize>, String> {
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
        .ok_or_else(|| {
            format!("the golden count of `{key}` is not a whole number of zero or more")
        })?;
    Ok((!undecided).then_some(count))
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

/// The `#unknown` counts of the golden `rules`, by rule. An `#unknown` key not of `idiomatic.arch/<rule>`, or a count that is not
/// a whole number of zero or more, fails.
pub(crate) fn unknowns_in(rules: &toml::Table) -> BTreeMap<String, usize> {
    rules
        .iter()
        .filter_map(|(key, count)| Some((key.strip_suffix("#unknown")?, key, count)))
        .map(|(rule, key, count)| {
            let rule = rule.strip_prefix("idiomatic.arch/").unwrap_or_else(|| {
                panic!("the golden key `{key}` names no `idiomatic.arch/` rule")
            });
            let count = count
                .as_integer()
                .and_then(|count| usize::try_from(count).ok())
                .unwrap_or_else(|| {
                    panic!("the golden count of `{key}` is not a whole number of zero or more")
                });
            (rule.to_owned(), count)
        })
        .collect()
}
