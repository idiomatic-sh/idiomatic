//! What the replay adds to the base: what a recording expects, one rule's rows compared with it, and the golden findings and the
//! recorded differences read.

use super::base::{required_text, text};
use std::collections::BTreeMap;
use std::path::Path;

/// What the recording of a rule expects.
pub(crate) struct Expectation {
    pub plan: String,
    pub expected: Vec<String>,
    /// Whether today's ryo could not decide the rule, so that nothing was recorded to compare with.
    pub undecided: bool,
    /// The findings of ruff checked by hand against the source (`checked.toml`): a rule with none is checked to give none.
    pub checked: Vec<String>,
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
