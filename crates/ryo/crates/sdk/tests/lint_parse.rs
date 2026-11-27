//! What a lint says of the files it could not read or decide (group 3b: a file that does not parse, with the answers the OLD engine gave on the trunk, recorded and asserted exactly).

use ryo::linter::{
    Cache, Corpus, Languages, LintReport, LintRequest, Linter, Rule, RuleApplicability, RuleId, SourceLanguage, WorkerPool, Workers,
};
use ryo::query::RyoQlQuery;
use std::collections::BTreeMap;
use std::num::NonZeroUsize;
use std::path::Path;

/// The files below `root`, relative, as a host lists them.
fn listed(root: &Path) -> Vec<ryo::linter::RelativePath> {
    fn walk(dir: &Path, base: &Path, out: &mut Vec<String>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir).expect("a folder").map(|entry| entry.expect("an entry").path()).collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                walk(&path, base, out);
            } else {
                out.push(path.strip_prefix(base).expect("below the root").to_string_lossy().replace('\\', "/"));
            }
        }
    }
    let mut found = Vec::new();
    walk(root, root, &mut found);
    found.into_iter().map(|path| ryo::linter::RelativePath(path.into())).collect()
}

const COMMENTS: &str = r#"let rows = select * from "comment";
let named = extend(from=rows, values={"problem": "a comment"});
emit named subject "file" key("span");"#;

fn linter() -> Linter {
    Linter::new(Languages(vec![ryo_toml::linked()]), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::None)
}

fn rule(query: &str) -> Rule {
    Rule {
        observations: Vec::new(),
        query: RyoQlQuery::try_from(query).expect("the query is one"),
        applicability: RuleApplicability::Always,
        parameters: BTreeMap::new(),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

fn request(root: &Path, rules: Vec<(&str, Rule)>) -> LintRequest {
    LintRequest {
        source_language: SourceLanguage::Recognized,
        root: root.to_path_buf(),
        project_search_scope: Default::default(),
        rules: rules.into_iter().map(|(id, rule)| (RuleId::try_from(id).expect("an id"), rule)).collect(),
        files: Vec::new(),
        hosted: Vec::new(),
        // The executor reads the files the host lists (a closed inventory): `Corpus::Walked` is refused by it (see `walked_corpus_is_refused`).
        corpus: Corpus::Listed(listed(root)),
        beside: Vec::new(),
    }
}

fn workspace(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    for (path, text) in files {
        let at = root.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a folder");
        std::fs::write(at, text).expect("a file");
    }
    root
}

fn lint(request: LintRequest) -> LintReport {
    linter().lint(vec![request]).pop().expect("one answer").expect("the lint is answered")
}

const BAD: &str = "this is = = not toml [[\n";

fn unknowns(report: &LintReport) -> Vec<(String, String)> {
    let mut found: Vec<(String, String)> = report.unknowns.iter().map(|unknown| (format!("{:?}", unknown.path), format!("{:?}", unknown.message))).collect();
    found.sort();
    found
}

/// OLD (trunk 4683a9657, recorded): a rule that always applies, over a file that does not parse: no finding, nothing unread, and ONE
/// unknown: `comment is read only in part in bad.toml: the file has syntax errors`.
#[test]
fn a_file_that_does_not_parse_leaves_an_unknown_for_the_relation_read_in_part() {
    let root = workspace(&[("bad.toml", BAD)]);
    let report = lint(request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]));
    assert!(report.findings.is_empty() && report.unread.is_empty() && report.skipped.is_empty(), "{report:?}");
    assert_eq!(
        unknowns(&report),
        vec![("RelativePath(\"bad.toml\")".to_owned(), "Message(\"comment is read only in part in bad.toml: the file has syntax errors\")".to_owned())],
        "{report:?}"
    );
}

/// Decided 2026-11-26: per-file; the old engine withheld the rule from every file of the language. (Recorded on the trunk 4683a9657:) a rule that needs accepted syntax over TOML was, in the old engine, an
/// unknown `rule skipped: syntax acceptance undetermined` for EVERY file (the good one too), with nothing in `skipped` and no finding.
/// The executor decides acceptance: the good file is linted (one finding) and the file that does not parse is in `skipped`.
#[test]
fn a_rule_that_needs_accepted_syntax_is_decided_per_file_by_the_executor() {
    let root = workspace(&[("good.toml", "# one\n"), ("bad.toml", BAD)]);
    let mut accepted = rule(COMMENTS);
    accepted.applicability = RuleApplicability::AcceptedSyntax;
    let report = lint(request(root.path(), vec![("idiomatic/comments", accepted)]));
    assert_eq!(report.findings.len(), 1, "{report:?}");
    assert!(report.skipped.iter().any(|skipped| format!("{skipped:?}").contains("bad.toml")), "{report:?}");
    assert!(report.unknowns.is_empty(), "{report:?}");
}
