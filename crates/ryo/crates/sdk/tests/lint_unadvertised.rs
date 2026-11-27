//! A rule over an optional relation the provider does not advertise (`binding-event`, which TOML does not read): the old engine ran it
//! and answered an unknown at each file, never "no finding"; the rules beside it ran. A relation nobody declares stays refused.

use ryo::linter::{
    Cache, Corpus, Languages, LintReport, LintRequest, Linter, Rule, RuleApplicability, RuleId, SourceLanguage, WorkerPool, Workers};
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

const EVENTS: &str = r#"let rows = select * from "binding-event";
let named = extend(from=rows, values={"problem": "an event"});
emit named subject "file" key("span");"#;

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

const UNAVAILABLE: &str = "The requested optional capability is unavailable from this provider.";

fn lint_in(store: Option<&Path>, root: &Path) -> LintReport {
    let cache = store.map_or(Cache::None, |path| Cache::At(path.to_path_buf()));
    let linter = Linter::new(Languages(vec![ryo_toml::linked()]), WorkerPool::from(Workers(NonZeroUsize::MIN)), cache);
    let asked = request(root, vec![("idiomatic/comments", rule(COMMENTS)), ("idiomatic/events", rule(EVENTS))]);
    linter.lint(vec![asked]).pop().expect("one answer").expect("nothing is refused")
}

fn summary(report: &LintReport) -> (usize, Vec<(String, String)>) {
    let mut unknowns: Vec<_> = report.unknowns.iter().map(|unknown| (format!("{:?}", unknown.rule), format!("{:?}", unknown.message))).collect();
    unknowns.sort();
    (report.findings.len(), unknowns)
}

/// Old: every rule over an optional capability the provider does not advertise ran and gave an unknown at the file; the ordinary rule gave
/// its finding; nothing was unread or refused.
#[test]
fn a_rule_over_an_unadvertised_relation_is_an_unknown_beside_the_rules_that_run() {
    let root = tempfile::tempdir().expect("a directory");
    std::fs::write(root.path().join("a.toml"), "# one\n").expect("a file");
    let report = lint_in(None, root.path());
    let (findings, unknowns) = summary(&report);
    assert_eq!(findings, 1, "{report:?}");
    assert_eq!(unknowns, vec![("RuleId(\"idiomatic/events\")".to_owned(), format!("Message({:?})", format!("binding-event is read only in part in a.toml: {UNAVAILABLE}")))], "{report:?}");
    assert!(report.unread.is_empty(), "{report:?}");
}

/// Cold and warm give the same answer: the unknown is stored and replayed like any other.
#[test]
fn the_unknown_is_the_same_cold_and_warm() {
    let root = tempfile::tempdir().expect("a directory");
    std::fs::write(root.path().join("a.toml"), "# one\n").expect("a file");
    let store = tempfile::tempdir().expect("a store");
    let cold = lint_in(Some(store.path()), root.path());
    let warm = lint_in(Some(store.path()), root.path());
    assert_eq!(summary(&cold), summary(&warm), "{cold:?} {warm:?}");
    assert_eq!(summary(&cold).0, 1);
}
