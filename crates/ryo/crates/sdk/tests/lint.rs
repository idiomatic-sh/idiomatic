//! What a lint answers for a request, through the SDK's public `Linter::lint`, over small workspaces on disk read by a real language
//! (TOML): the observable behaviour of the lint that the old per-file engine's use-case tests held, re-pointed here. Each test names
//! the behaviour; where the executor answers otherwise than the old engine did, the test says what the old engine answered.

use ryo::linter::{
    Cache, Corpus, Glob, Languages, LintReport, LintRequest, Linter, Rule, RuleApplicability, RuleId, SourceLanguage, WorkerPool, Workers,
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

/// A rule that reports every comment of a TOML file, with `problem` as its message.
const COMMENTS: &str = r#"let rows = select * from "comment";
let named = extend(from=rows, values={"problem": "a comment"});
emit named subject "file" key("span");"#;

/// A rule that reports every key of a TOML file named `bad`.
const BAD_KEYS: &str = r#"let rows = select * from "entry";
let bad = select * from rows where key = "bad";
let named = extend(from=bad, values={"problem": "a key named bad"});
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

/// The files the findings are about, in the order of the report.
fn files_of(report: &LintReport) -> Vec<String> {
    let mut files: Vec<String> = report.findings.iter().map(|finding| format!("{:?}", finding.subject)).collect();
    files.sort();
    files
}

// ---- group 1: findings ----

/// Old: `violation_is_reported_with_path_and_span`.
#[test]
fn a_violation_is_reported_with_its_file_and_its_place() {
    let root = workspace(&[("a.toml", "# one\nname = \"x\"\n")]);
    let report = lint(request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]));
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    let finding = &report.findings[0];
    assert!(format!("{:?}", finding.subject).contains("a.toml"), "{:?}", finding.subject);
    assert!(format!("{:?}", finding.place).contains("line: Line(1)"), "{:?}", finding.place);
}

/// Old: `clean_repository_has_no_findings`.
#[test]
fn a_clean_repository_has_no_findings() {
    let root = workspace(&[("a.toml", "name = \"x\"\n"), ("b.toml", "[t]\nk = 1\n")]);
    let report = lint(request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]));
    assert!(report.findings.is_empty(), "{:?}", report.findings);
    assert!(report.unread.is_empty(), "{:?}", report.unread);
}

/// Old: `lint_is_scoped_to_the_files_given`.
#[test]
fn a_lint_given_files_reports_findings_in_those_files_only() {
    let root = workspace(&[("a.toml", "# one\n"), ("b.toml", "# two\n")]);
    let mut asked = request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]);
    asked.files = vec![Glob::try_from("a.toml").expect("a glob")];
    let report = lint(asked);
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    assert!(format!("{:?}", report.findings[0].subject).contains("a.toml"));
}

/// Old: `a_rule_does_not_govern_the_files_it_leaves_out` and `a_rule_governs_only_the_files_it_includes_and_the_others_are_not_read`.
#[test]
fn a_rule_governs_only_the_files_it_includes() {
    let root = workspace(&[("a.toml", "# one\n"), ("b.toml", "# two\n")]);
    let mut governed = rule(COMMENTS);
    governed.include = vec![Glob::try_from("b.toml").expect("a glob")];
    let report = lint(request(root.path(), vec![("idiomatic/comments", governed)]));
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    assert!(format!("{:?}", report.findings[0].subject).contains("b.toml"));
    let mut left_out = rule(COMMENTS);
    left_out.exclude = vec![Glob::try_from("a.toml").expect("a glob")];
    let report = lint(request(root.path(), vec![("idiomatic/comments", left_out)]));
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    assert!(format!("{:?}", report.findings[0].subject).contains("b.toml"));
}

/// Old: `two_rules_answer_every_row_of_each`.
#[test]
fn two_rules_each_answer_their_own_findings() {
    let root = workspace(&[("a.toml", "# one\nbad = 1\n")]);
    let report = lint(request(
        root.path(),
        vec![("idiomatic/comments", rule(COMMENTS)), ("idiomatic/bad-keys", rule(BAD_KEYS))],
    ));
    let by_rule: BTreeMap<String, usize> = report.findings.iter().fold(BTreeMap::new(), |mut each, finding| {
        *each.entry(finding.rule.to_string()).or_default() += 1;
        each
    });
    assert_eq!(by_rule.get("idiomatic/comments"), Some(&1), "{by_rule:?}");
    assert_eq!(by_rule.get("idiomatic/bad-keys"), Some(&1), "{by_rule:?}");
    assert_eq!(files_of(&report).len(), 2);
}
