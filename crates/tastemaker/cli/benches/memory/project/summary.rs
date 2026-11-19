//! A target's expected findings as a summary: the sha256 digest of the command's whole
//! findings document in canonical form (object keys sorted, every array's elements sorted),
//! with only boolean `tools[].reused` cache provenance normalized to false, the exit status
//! the run must end with, and the count per rule. A run is held to the digest exactly; on a
//! mismatch the per-rule changes name what moved. The total is the sum of the counts, never
//! stored beside them, and a summary with no finding has no `[rules]` table.

use sha2::{Digest as _, Sha256};
use std::collections::BTreeMap;
use std::path::Path;

/// What a run's findings are held to.
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct Summary {
    /// The sha256 of the whole findings document in canonical form, which a run must match exactly.
    digest: String,
    /// The exit code the command must end with over the target.
    exit_code: i32,
    /// How many findings each rule reported, which names what moved when the digest differs.
    rules: BTreeMap<String, u64>,
}

/// `value` in canonical form: object keys in order, each array's elements sorted by their
/// own canonical text, so the digest does not depend on the order findings are printed in.
fn canonical(value: &serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut entries: Vec<_> = map.iter().collect();
            entries.sort_by(|left, right| left.0.cmp(right.0));
            serde_json::Value::Object(
                entries
                    .into_iter()
                    .map(|(key, member)| (key.clone(), canonical(member)))
                    .collect(),
            )
        }
        serde_json::Value::Array(items) => {
            let mut items: Vec<(String, serde_json::Value)> = items
                .iter()
                .map(|item| {
                    let item = canonical(item);
                    (item.to_string(), item)
                })
                .collect();
            items.sort_by(|left, right| left.0.cmp(&right.0));
            serde_json::Value::Array(items.into_iter().map(|(_, item)| item).collect())
        }
        other @ (serde_json::Value::Null
        | serde_json::Value::Bool(_)
        | serde_json::Value::Number(_)
        | serde_json::Value::String(_)) => other.clone(),
    }
}

/// The summary of what a command printed: its whole findings document, and the `rule` of
/// every entry of its top-level arrays that names one, whatever the product calls them
/// (Tastemaker's `findings` and `suppressed`, asymptote's `new` and the rest), with the
/// `exit_code` the run ended with.
pub(crate) fn of(printed: &[u8], exit_code: i32) -> std::io::Result<Summary> {
    let mut document: serde_json::Value =
        serde_json::from_slice(printed).map_err(std::io::Error::other)?;
    // Reusing an unchanged tool result is the warm run's expected behavior. Keep tool
    // identity, status, errors, findings and malformed reuse values in the exact digest.
    if let Some(tools) = document
        .get_mut("tools")
        .and_then(serde_json::Value::as_array_mut)
    {
        for tool in tools {
            if let Some(reused) = tool.get_mut("reused").filter(|value| value.is_boolean()) {
                *reused = serde_json::Value::Bool(false);
            }
        }
    }
    let digest = Sha256::digest(canonical(&document).to_string().as_bytes())
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .concat();
    let mut rules = BTreeMap::new();
    let entries = document
        .as_object()
        .into_iter()
        .flat_map(serde_json::Map::values)
        .filter_map(serde_json::Value::as_array)
        .flatten();
    for rule in entries.filter_map(|entry| entry.get("rule").and_then(serde_json::Value::as_str)) {
        *rules.entry(rule.to_owned()).or_insert(0) += 1;
    }
    Ok(Summary {
        digest,
        exit_code,
        rules,
    })
}

/// The summary a target's `findings.toml` or `edit-findings.toml` states.
pub(crate) fn read(file: &Path) -> std::io::Result<Summary> {
    let table: toml::Table = std::fs::read_to_string(file)?
        .parse()
        .map_err(|error| std::io::Error::other(format!("{}: {error}", file.display())))?;
    let invalid = |what: &str| std::io::Error::other(format!("{}: {what}", file.display()));
    let digest = table
        .get("digest")
        .and_then(toml::Value::as_str)
        .filter(|digest| digest.len() == 64 && digest.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .ok_or_else(|| invalid("`digest` is not a sha256 in hex"))?;
    let count = |value: &toml::Value| {
        let count = value.as_integer()?;
        u64::try_from(count).ok()
    };
    if let Some(key) = table
        .keys()
        .find(|key| !["digest", "exit_code", "rules"].contains(&key.as_str()))
    {
        return Err(invalid(&format!("`{key}` is not a key of a summary")));
    }
    let exit_code = table
        .get("exit_code")
        .and_then(toml::Value::as_integer)
        .and_then(|code| i32::try_from(code).ok())
        .ok_or_else(|| invalid("`exit_code` is not an exit status"))?;
    let none = toml::Table::new();
    let rules = match table.get("rules") {
        None => &none,
        Some(rules) => rules
            .as_table()
            .ok_or_else(|| invalid("`rules` is not a table"))?,
    }
    .iter()
    .map(|(rule, value)| {
        count(value)
            .map(|counted| (rule.clone(), counted))
            .ok_or_else(|| invalid("a rule's count is not a count"))
    })
    .collect::<std::io::Result<BTreeMap<String, u64>>>()?;
    Ok(Summary {
        digest: digest.to_owned(),
        exit_code,
        rules,
    })
}

impl Summary {
    /// Whether the run printed exactly the expected findings.
    pub(crate) fn matches(&self, expected: &Self) -> bool {
        self.digest == expected.digest
    }

    /// How many findings the run printed: the sum of the counts per rule.
    pub(crate) fn total(&self) -> u64 {
        self.rules.values().sum()
    }

    /// The exit status the run must end with.
    pub(crate) fn exit_code(&self) -> i32 {
        self.exit_code
    }

    /// The summary as its file states it: no `[rules]` table when no rule found anything.
    pub(crate) fn text(&self) -> String {
        let mut text = format!(
            "# The findings the command prints over this target: the sha256 of the whole\n\
             # findings document in canonical form, the exit status the run must end with\n\
             # and their count per rule.\n\
             digest = \"{}\"\nexit_code = {}\n",
            self.digest, self.exit_code
        );
        if !self.rules.is_empty() {
            text.push_str("\n[rules]\n");
        }
        let rules: Vec<String> = self
            .rules
            .iter()
            .map(|(rule, count)| format!("{} = {count}\n", toml::Value::from(rule.as_str())))
            .collect();
        text.push_str(&rules.concat());
        text
    }

    /// One line per rule whose count changed, `<rule> <expected> → <actual> (±n)`, then
    /// the total's.
    pub(crate) fn changes(&self, expected: &Self) -> Vec<String> {
        let line = |name: &str, before: u64, after: u64| {
            let change = i128::from(after) - i128::from(before);
            format!("{name} {before} → {after} ({change:+})")
        };
        let mut rules: Vec<&String> = expected.rules.keys().chain(self.rules.keys()).collect();
        rules.sort();
        rules.dedup();
        let mut lines: Vec<String> = rules
            .into_iter()
            .filter_map(|rule| {
                let before = expected.rules.get(rule).copied().unwrap_or(0);
                let after = self.rules.get(rule).copied().unwrap_or(0);
                (before != after).then(|| line(rule, before, after))
            })
            .collect();
        lines.push(line("total", expected.total(), self.total()));
        lines
    }
}
