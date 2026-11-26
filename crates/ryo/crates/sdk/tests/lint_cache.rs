//! What a lint answers across runs over one store (groups 5 and 6 of the re-pointed lint behaviour: see `tests/lint.rs`).

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

/// A rule that reports every comment of a TOML file, with `problem` as its message.
const COMMENTS: &str = r#"let rows = select * from "comment";
let named = extend(from=rows, values={"problem": "a comment"});
emit named subject "file" key("span");"#;

fn linter(store: &Path) -> Linter {
    Linter::new(Languages(vec![ryo_toml::linked()]), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::At(store.to_path_buf()))
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

/// One lint over `root` with a linter of its own over the store at `store`: as a second process would.
fn lint_in(store: &Path, request: LintRequest) -> Result<LintReport, ryo::linter::LintError> {
    linter(store).lint(vec![request]).pop().expect("one answer")
}

fn comments_at(root: &Path) -> LintRequest {
    request(root, vec![("idiomatic/comments", rule(COMMENTS))])
}

fn subjects(report: &LintReport) -> Vec<String> {
    let mut found: Vec<String> = report.findings.iter().map(|finding| format!("{:?}", finding.subject)).collect();
    found.sort();
    found
}

// ---- group 5: cache lifecycle (the cache survives: `Linter::new(languages, pool, cache)`) ----

/// Old: `cold_warm_and_edited_runs_answer_the_same_in_every_store`, `unchanged_reuse_reads_no_file` (the answer side),
/// `a_body_edit_reads_only_that_file_and_keeps_the_findings`.
#[test]
fn cold_warm_and_edited_runs_answer_as_a_cold_run_over_the_same_files() {
    let root = workspace(&[("a.toml", "# one\n"), ("b.toml", "# two\n")]);
    let store = tempfile::tempdir().expect("a store");
    let cold = lint_in(store.path(), comments_at(root.path())).expect("cold");
    let warm = lint_in(store.path(), comments_at(root.path())).expect("warm");
    assert_eq!(cold.findings, warm.findings, "a warm run over unchanged files answers as the cold one");
    std::fs::write(root.path().join("a.toml"), "name = \"x\"\n").expect("an edit");
    let edited = lint_in(store.path(), comments_at(root.path())).expect("edited");
    let fresh = lint_in(tempfile::tempdir().expect("a store").path(), comments_at(root.path())).expect("fresh");
    assert_eq!(edited.findings, fresh.findings, "an edited run answers as a run with no store");
    assert_eq!(subjects(&edited).len(), 1, "{:?}", edited.findings);
}

/// When `lint` returns, everything the cache is to keep is on disk: a host may end its process without running destructors (the
/// linter forgotten here) and lose nothing, and a second linter over the same store answers as the first did.
#[test]
fn a_lint_leaves_what_the_cache_keeps_on_disk_before_it_returns() {
    let root = workspace(&[("a.toml", "# one\n"), ("b.toml", "# two\n")]);
    let store = tempfile::tempdir().expect("a store");
    let first = linter(store.path());
    let cold = first.lint(vec![comments_at(root.path())]).pop().expect("one answer").expect("cold");
    std::mem::forget(first);
    assert!(std::fs::read_dir(store.path()).expect("the store").next().is_some(), "the store is on disk with the linter forgotten");
    let warm = lint_in(store.path(), comments_at(root.path())).expect("warm");
    assert_eq!(cold.findings, warm.findings, "a second linter over the store answers as the first did");
}

/// Old: `a_file_whose_bytes_could_not_be_read_is_read_again_by_the_next_run`, deleted files.
#[test]
fn a_deleted_file_has_no_finding_in_the_next_run() {
    let root = workspace(&[("a.toml", "# one\n"), ("b.toml", "# two\n")]);
    let store = tempfile::tempdir().expect("a store");
    assert_eq!(lint_in(store.path(), comments_at(root.path())).expect("cold").findings.len(), 2);
    std::fs::remove_file(root.path().join("b.toml")).expect("a deletion");
    let next = lint_in(store.path(), comments_at(root.path())).expect("next");
    assert_eq!(next.findings.len(), 1, "{:?}", next.findings);
}

/// Old: `a_failed_run_keeps_the_previous_state`.
#[test]
fn a_refused_run_leaves_the_store_as_it_was() {
    let root = workspace(&[("a.toml", "# one\n")]);
    let store = tempfile::tempdir().expect("a store");
    let before = lint_in(store.path(), comments_at(root.path())).expect("cold");
    let mut bad = comments_at(root.path());
    bad.source_language = SourceLanguage::Named(ryo::languages::LanguageName("no-such-language".into()));
    assert!(lint_in(store.path(), bad).is_err());
    let after = lint_in(store.path(), comments_at(root.path())).expect("after");
    assert_eq!(before.findings, after.findings);
}

/// Old (`guards`): a store cut short, replaced by other bytes, with a byte flipped, or removed answers as if there were none.
#[test]
fn a_damaged_store_answers_as_if_there_were_none() {
    let root = workspace(&[("a.toml", "# one\n"), ("b.toml", "# two\n")]);
    let store = tempfile::tempdir().expect("a store");
    let cold = lint_in(store.path(), comments_at(root.path())).expect("cold");
    let kept: Vec<std::path::PathBuf> = {
        let mut found = Vec::new();
        let mut stack = vec![store.path().to_path_buf()];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).expect("a folder").flatten() {
                if entry.path().is_dir() {
                    stack.push(entry.path());
                } else {
                    found.push(entry.path());
                }
            }
        }
        found
    };
    for damage in 0..4 {
        for path in &kept {
            let Ok(bytes) = std::fs::read(path) else { continue };
            match damage {
                0 => std::fs::write(path, &bytes[..bytes.len() / 2]).expect("cut short"),
                1 => std::fs::write(path, b"other bytes entirely").expect("replaced"),
                2 => {
                    let mut flipped = bytes.clone();
                    if let Some(first) = flipped.first_mut() {
                        *first ^= 0xff;
                    }
                    std::fs::write(path, flipped).expect("flipped");
                }
                _ => std::fs::remove_file(path).expect("removed"),
            }
        }
        let after = lint_in(store.path(), comments_at(root.path())).expect("after damage");
        assert_eq!(cold.findings, after.findings, "damage {damage}: the answer is the cold one");
    }
}

// ---- group 6: an identity that does not apply ----

/// Old (`inapplicable_identity`): rules whose inapplicable files differ answer alike cold and warm.
#[test]
fn a_rule_that_needs_accepted_syntax_answers_alike_cold_and_warm() {
    let root = workspace(&[("good.toml", "# one\n"), ("bad.toml", "this is = = not toml [[\n")]);
    let store = tempfile::tempdir().expect("a store");
    let mut accepted = rule(COMMENTS);
    accepted.applicability = RuleApplicability::AcceptedSyntax;
    let ask = |root: &Path, accepted: Rule| request(root, vec![("idiomatic/comments", accepted)]);
    let cold = lint_in(store.path(), ask(root.path(), accepted.clone())).expect("cold");
    let warm = lint_in(store.path(), ask(root.path(), accepted)).expect("warm");
    assert_eq!(cold.findings, warm.findings);
    assert_eq!(cold.skipped, warm.skipped);
}
