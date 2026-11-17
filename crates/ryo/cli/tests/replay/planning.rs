//! Every rule ryo is given plans through the production plan, the plan's growth check among it: each rule of the Ruff,
//! oxlint and clippy benchmarks, the live version of every Tastemaker rule, and every query a test of this crate embeds.

use crate::loading::{self, Loaded, Production};
use std::collections::BTreeMap;
use std::path::Path;

/// Every string literal of the Rust files under `directory` that holds an `emit`, by its file and its place among them.
fn embedded_queries(directory: &Path) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut pending = vec![directory.to_path_buf()];
    while let Some(each) = pending.pop() {
        for entry in std::fs::read_dir(&each)
            .expect("a directory of tests")
            .flatten()
        {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|extension| extension == "rs") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
        .iter()
        .flat_map(|file| {
            let text = std::fs::read_to_string(file).expect("a test file");
            let name = file
                .strip_prefix(directory)
                .unwrap_or(file)
                .display()
                .to_string();
            string_literals(&text)
                .into_iter()
                .filter(|literal| literal.contains("emit "))
                .enumerate()
                .map(move |(at, literal)| (format!("{name}#{at}"), literal))
                .collect::<Vec<_>>()
        })
        .collect()
}

/// The string literals of Rust source `text`, raw and escaped, their escapes read; comments and character literals skipped.
fn string_literals(text: &str) -> Vec<String> {
    let bytes = text.as_bytes();
    let mut found = Vec::new();
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let next = bytes.get(at + 1).copied();
        at = match byte {
            b'/' if next == Some(b'/') => text[at..].find('\n').map_or(bytes.len(), |end| at + end),
            b'/' if next == Some(b'*') => text[at + 2..]
                .find("*/")
                .map_or(bytes.len(), |end| at + 2 + end + 2),
            b'\'' => past_character(text, at),
            b'r' if matches!(next, Some(b'"' | b'#')) && !in_word(bytes, at) => {
                raw_literal(text, at, &mut found)
            }
            b'"' => escaped_literal(text, at, &mut found),
            _ => at + 1,
        };
    }
    found
}

/// Whether the byte before `at` continues a word, so an `r` there is no raw literal's.
fn in_word(bytes: &[u8], at: usize) -> bool {
    at.checked_sub(1)
        .and_then(|before| bytes.get(before))
        .is_some_and(|before| before.is_ascii_alphanumeric() || *before == b'_')
}

/// Past the character literal at `at` (`'x'` or `'\…'`), or past the `'` of a lifetime.
fn past_character(text: &str, at: usize) -> usize {
    let bytes = text.as_bytes();
    match (bytes.get(at + 1), bytes.get(at + 2)) {
        (Some(b'\\'), _) => text[at + 2..]
            .find('\'')
            .map_or(bytes.len(), |end| at + 2 + end + 1),
        (Some(_), Some(b'\'')) => at + 3,
        _ => at + 1,
    }
}

/// The raw literal at `at` (`r"…"`, `r#"…"#`) into `found`; past it.
fn raw_literal(text: &str, at: usize, found: &mut Vec<String>) -> usize {
    let bytes = text.as_bytes();
    let hashes = bytes[at + 1..]
        .iter()
        .take_while(|each| **each == b'#')
        .count();
    let open = at + 1 + hashes;
    if bytes.get(open) != Some(&b'"') {
        return at + 1;
    }
    let close = format!("\"{}", "#".repeat(hashes));
    let end = text[open + 1..]
        .find(&close)
        .map_or(bytes.len(), |end| open + 1 + end);
    found.push(text[open + 1..end].to_owned());
    end + close.len()
}

/// The escaped literal at `at` into `found`, its escapes read and its line continuations joined; past it.
fn escaped_literal(text: &str, at: usize, found: &mut Vec<String>) -> usize {
    let mut literal = String::new();
    let mut rest = &text[at + 1..];
    loop {
        let Some(stop) = rest.find(['"', '\\']) else {
            found.push(literal);
            return text.len();
        };
        literal.push_str(&rest[..stop]);
        if rest[stop..].starts_with('"') {
            found.push(literal);
            return text.len() - rest.len() + stop + 1;
        }
        let mut escaped = rest[stop + 1..].chars();
        match escaped.next() {
            Some('n') => literal.push('\n'),
            Some('t') => literal.push('\t'),
            // A line continuation: the newline and the next line's leading whitespace go.
            Some('\n') => {}
            Some(other) => literal.push(other),
            None => {}
        }
        rest = escaped.as_str();
        if text[..text.len() - rest.len()].ends_with('\n') {
            rest = rest.trim_start();
        }
    }
}

/// Every rule ryo is given plans through the production plan, its growth check among it: no rule's plan holds a node that
/// pairs the rows of a file with themselves where its query did not declare it with `all-pairs join`. A benchmark or
/// Tastemaker rule that does not load for any other reason fails as well. A query a test embeds that cannot load alone (it
/// reads a relation, a parameter or an observation its test supplies) is planned by its own test, and only a refusal of
/// the growth check fails here.
#[test]
fn every_rule_plans_through_the_production_plan() {
    let production = Production::new();
    let mut refused: Vec<String> = Vec::new();
    let mut planned = BTreeMap::<&str, usize>::new();
    for linter in loading::BENCHMARKS {
        let rules = loading::benchmark_rules(&loading::workspace().join(linter));
        assert!(!rules.is_empty(), "{linter} has rules");
        for (each, hosted) in &rules {
            match production.load(each, hosted) {
                Loaded::Planned => *planned.entry(linter).or_default() += 1,
                Loaded::Unloaded(reason) => {
                    refused.push(format!("{linter} {}: {reason}", each.id.0));
                }
            }
        }
    }
    let rules = loading::tastemaker_rules();
    assert!(!rules.is_empty(), "the Tastemaker package has rules");
    for (name, each, hosted) in &rules {
        match production.load(each, hosted) {
            Loaded::Planned => *planned.entry("tastemaker").or_default() += 1,
            Loaded::Unloaded(reason) => {
                refused.push(format!("tastemaker {name}: {reason}"));
            }
        }
    }
    let mut alone: Vec<&str> = Vec::new();
    let queries = embedded_queries(&Path::new(env!("CARGO_MANIFEST_DIR")).join("tests"));
    assert!(!queries.is_empty(), "the tests embed queries");
    for (name, query) in &queries {
        match production.load(&loading::query_rule(name, query), &[]) {
            Loaded::Planned => *planned.entry("tests").or_default() += 1,
            Loaded::Unloaded(_) => alone.push(name),
        }
    }
    eprintln!("planned: {planned:?}; embedded queries that do not load alone: {alone:?}");
    assert!(
        refused.is_empty(),
        "{} rules do not plan:\n{}",
        refused.len(),
        refused.join("\n")
    );
}
