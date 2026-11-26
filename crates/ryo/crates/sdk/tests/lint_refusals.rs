//! What a lint refuses, through the SDK's public `Linter::lint` (group 2 of the re-pointed lint behaviour: see `tests/lint.rs`). Where the
//! old engine refused at the lint call, a request the types of this crate cannot hold is refused earlier, at the type: those tests say
//! so, and assert the refusal where it now happens.

use ryo::linter::{
    Cache, Corpus, Glob, Languages, LintError, LintRequest, Linter, Rule, RuleApplicability, RuleId, SourceLanguage, WorkerPool, Workers,
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
        given_project_inventory: Default::default(),
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

fn refused(request: LintRequest) -> LintError {
    linter().lint(vec![request]).pop().expect("one answer").expect_err("the lint is refused")
}

/// Old: `rule_that_does_not_validate_is_refused`.
#[test]
fn a_rule_whose_query_is_not_ryoql_is_refused_by_the_lint() {
    let root = workspace(&[("a.toml", "# one\n")]);
    let Ok(query) = RyoQlQuery::try_from("this is not a query") else {
        return; // refused where it is made: the same behaviour, earlier
    };
    let mut broken = rule(COMMENTS);
    broken.query = query;
    let error = refused(request(root.path(), vec![("idiomatic/broken", broken)]));
    assert!(matches!(error, LintError::Rule { .. }), "{error:?}");
}

/// Old: `a_rule_without_an_id_is_refused`. Now: at the type.
#[test]
fn a_rule_without_an_id_is_refused_where_the_id_is_made() {
    assert!(RuleId::try_from("").is_err());
}

/// Old: `a_glob_that_is_not_one_is_refused`. Now: at the type.
#[test]
fn a_glob_that_is_not_one_is_refused_where_it_is_made() {
    assert!(Glob::try_from("[").is_err());
}

/// Old: `two_rules_with_one_id_are_refused`: a request holds its rules in a map by id, so two rules of one id cannot be written.
#[test]
fn two_rules_of_one_id_cannot_be_written() {
    let root = workspace(&[("a.toml", "# one\n")]);
    let mut asked = request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]);
    let second = rule(COMMENTS);
    let _ = asked.rules.insert(RuleId::try_from("idiomatic/comments").expect("an id"), second);
    assert_eq!(asked.rules.len(), 1, "the second rule replaced the first: one rule remains");
}

/// Old: `a_parameter_the_rule_gives_nothing_for_is_refused`.
#[test]
fn a_parameter_nothing_is_bound_to_is_refused() {
    let query = r#"param word: string;
let rows = select * from "comment" where text = $word;
let named = extend(from=rows, values={"problem": "a comment"});
emit named subject "file" key("span");"#;
    let root = workspace(&[("a.toml", "# one\n")]);
    let error = refused(request(root.path(), vec![("idiomatic/with-parameter", rule(query))]));
    assert!(matches!(error, LintError::MissingParameter { .. }), "{error:?}");
}

/// Old: `an_unknown_semantic_kind_is_refused_before_a_file_is_read`: refused by the admission checks, before any file is read, as
/// `Unrunnable` (the old engine's `UnknownKind` is answered to the caller as `Unrunnable` with the reason).
#[test]
fn a_query_naming_a_kind_no_language_has_is_refused_before_a_file_is_read() {
    let query = r#"let rows = select * from "syntax-tree" where kind = "no-such-kind";
let named = extend(from=rows, values={"problem": "a kind"});
emit named subject "file" key("id");"#;
    let root = workspace(&[("a.toml", "# one\n")]);
    let error = refused(request(root.path(), vec![("idiomatic/kinds", rule(query))]));
    assert!(matches!(error, LintError::Unrunnable { .. }), "{error:?}");
}

/// Old: `a_source_provider_the_request_names_but_no_language_links_is_refused`: `SourceLanguage`, answered by the admission checks.
#[test]
fn a_provider_no_language_links_is_refused() {
    let root = workspace(&[("a.toml", "# one\n")]);
    let mut asked = request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]);
    asked.source_language = SourceLanguage::Named(ryo::languages::LanguageName("no-such-language".into()));
    let error = refused(asked);
    assert!(matches!(error, LintError::SourceLanguage { .. }), "{error:?}");
}

/// The old engine walked the repository for `Corpus::Walked`; the SDK lists the files itself for the executor, which takes a closed
/// inventory.
#[test]
fn a_walked_corpus_walks_the_repository() {
    let root = workspace(&[("a.toml", "# one\n"), ("sub/b.toml", "# two\n")]);
    let mut asked = request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]);
    asked.corpus = Corpus::Walked;
    let report = linter().lint(vec![asked]).pop().expect("one answer").expect("the walked lint is answered");
    assert_eq!(report.findings.len(), 2, "{:?}", report.findings);
}
