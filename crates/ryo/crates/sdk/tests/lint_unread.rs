//! What a lint says of the files it could not read or decide (group 3 of the re-pointed lint behaviour: see `tests/lint.rs`).

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

fn lint(request: LintRequest) -> LintReport {
    linter().lint(vec![request]).pop().expect("one answer").expect("the lint is answered")
}

/// Old: `unsupported_file_is_unread_without_a_failure` and `unread_files_are_reported`.
#[test]
fn a_file_no_language_reads_is_reported_unread_and_is_no_failure() {
    let root = workspace(&[("a.toml", "# one\n"), ("notes.xyz", "nothing reads this\n")]);
    let report = lint(request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]));
    assert_eq!(report.findings.len(), 1, "{:?}", report.findings);
    assert!(report.unread.iter().any(|unread| format!("{unread:?}").contains("notes.xyz")), "{:?}", report.unread);
}

/// Old: `language_parse_failure_is_unread_not_empty`: a file the language fails to parse is unread, never an empty read.
#[test]
fn a_file_that_does_not_parse_is_unread_or_partial_never_a_clean_empty() {
    let root = workspace(&[("bad.toml", "this is = = not toml [[\n")]);
    let report = lint(request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]));
    let said_so = report.unread.iter().any(|unread| format!("{unread:?}").contains("bad.toml"))
        || report.unknowns.iter().any(|unknown| format!("{unknown:?}").contains("bad.toml"))
        || !report.findings.is_empty();
    assert!(said_so, "the file is neither unread nor an unknown nor a finding: unread {:?}, unknowns {:?}", report.unread, report.unknowns);
}

/// Old: `a_syntax_skipped_file_does_not_poison_unrelated_partitions_cold_or_warm`.
#[test]
fn a_rule_that_needs_accepted_syntax_is_skipped_for_a_file_that_was_not_and_not_for_the_others() {
    let root = workspace(&[("good.toml", "# one\n"), ("bad.toml", "this is = = not toml [[\n")]);
    let mut accepted = rule(COMMENTS);
    accepted.applicability = RuleApplicability::AcceptedSyntax;
    let report = lint(request(root.path(), vec![("idiomatic/comments", accepted)]));
    assert!(report.findings.iter().any(|finding| format!("{:?}", finding.subject).contains("good.toml")), "{:?}", report.findings);
    assert!(report.skipped.iter().any(|skipped| format!("{skipped:?}").contains("bad.toml")), "{:?}", report.skipped);
}

/// Old: `accepted_files_answer_as_a_rule_that_always_applies`.
#[test]
fn files_with_accepted_syntax_answer_as_a_rule_that_always_applies() {
    let root = workspace(&[("a.toml", "# one\n"), ("b.toml", "# two\n")]);
    let mut accepted = rule(COMMENTS);
    accepted.applicability = RuleApplicability::AcceptedSyntax;
    let always = lint(request(root.path(), vec![("idiomatic/comments", rule(COMMENTS))]));
    let only_accepted = lint(request(root.path(), vec![("idiomatic/comments", accepted)]));
    assert_eq!(always.findings, only_accepted.findings);
    assert!(only_accepted.skipped.is_empty(), "{:?}", only_accepted.skipped);
}

/// A rule over the host's read failures: one finding for each file the host could not read.
const FAILURES: &str = r#"let failed = select * from "filesystem.read-failure";
emit failed subject "path" key("path");"#;

/// A file the host lists and cannot read is run as a host-only file: its `filesystem.read-failure` row is the host's, the file is named
/// unread with the reason, and every other relation a rule reads of it is read only in part, one unknown for each rule: cold and warm alike.
#[test]
fn a_listed_file_that_cannot_be_read_gives_its_read_failure_its_unread_entry_and_its_partial_notes() {
    let root = workspace(&[("a.toml", "# one\n")]);
    let store = tempfile::tempdir().expect("a store");
    let mut asked = request(root.path(), vec![("idiomatic/comments", rule(COMMENTS)), ("idiomatic/failures", rule(FAILURES))]);
    let Corpus::Listed(files) = &mut asked.corpus else { unreachable!("a listed corpus") };
    files.push(ryo::linter::RelativePath("missing.toml".into()));
    let over_store = |request: LintRequest| {
        Linter::new(Languages(vec![ryo_toml::linked()]), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::At(store.path().to_path_buf()))
            .lint(vec![request])
            .pop()
            .expect("one answer")
            .expect("the lint is answered")
    };
    let cold = over_store(asked.clone());
    let warm = over_store(asked.clone());
    for (phase, report) in [("cold", &cold), ("warm", &warm)] {
        let said = |text: &str| format!("{:?}", report.findings).contains(text);
        assert!(report.findings.iter().any(|f| format!("{:?}", f.subject).contains("a.toml")), "{phase}: the readable file's finding stays: {:?}", report.findings);
        assert!(said("missing.toml"), "{phase}: the read-failure row of the file the host could not read: {:?}", report.findings);
        assert_eq!(
            report.unread.iter().filter(|unread| format!("{unread:?}").contains("missing.toml")).count(),
            1,
            "{phase}: the file is named unread once, with the reason: {:?}",
            report.unread
        );
        assert!(
            report.unknowns.iter().any(|unknown| format!("{unknown:?}").contains("missing.toml") && format!("{unknown:?}").contains("idiomatic/comments")),
            "{phase}: the rule that reads the file's comments says it read them only in part: {:?}",
            report.unknowns
        );
    }
    assert_eq!(cold.findings, warm.findings, "a warm run answers what a cold one did");
}

/// A rule over the files the host observed.
const FILES: &str = r#"let failed = select path from "filesystem.read-failure";
let present = select path from "filesystem.file";
let both = join(left=failed, right=present, left-on=[path], right-on=[path], output={path: [left, path]});
emit both subject path key(path);"#;

/// The host wrote `filesystem.file` only for a file it observed: a file that is there and cannot be read has its row (and its read
/// failure); a file that is not there has no row, and the relation is noted read only in part.
#[test]
fn a_missing_file_has_no_filesystem_file_row_and_a_locked_one_has_its_row() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = workspace(&[("a.toml", "# one\n"), ("locked.toml", "# locked\n")]);
    std::fs::set_permissions(root.path().join("locked.toml"), std::fs::Permissions::from_mode(0o000)).expect("a mode");
    let locked_is_unreadable = std::fs::read(root.path().join("locked.toml")).is_err();
    let mut asked = request(root.path(), vec![("idiomatic/files", rule(FILES))]);
    let Corpus::Listed(listed) = &mut asked.corpus else { unreachable!("a listed corpus") };
    listed.push(ryo::linter::RelativePath("missing.toml".into()));
    let report = lint(asked);
    let rows = format!("{:?}", report.findings);
    assert!(!rows.contains("a.toml"), "the readable file has no read failure: {rows}");
    assert!(!rows.contains("missing.toml"), "a file that is not there has no filesystem.file row to join its failure with: {rows}");
    assert!(
        report.unknowns.iter().any(|unknown| format!("{unknown:?}").contains("missing.toml")),
        "the relation is read only in part in the file that is not there: {:?}",
        report.unknowns
    );
    if locked_is_unreadable {
        assert!(rows.contains("locked.toml"), "a file that is there and locked has its row: {rows}");
    }
    std::fs::set_permissions(root.path().join("locked.toml"), std::fs::Permissions::from_mode(0o644)).expect("a mode");
}

/// A rule over the file inventory alone.
const INVENTORY: &str = r#"let present = select path from "filesystem.file";
emit present subject path key(path);"#;

/// A program over `filesystem.file` alone answers from the listed inventory: a listed file that is not there has no row (the relation is
/// noted read only in part for it), and a file that is there and locked keeps its row: cold and warm alike.
#[test]
fn an_inventory_program_has_no_row_for_a_listed_missing_file_and_keeps_the_locked_one_cold_and_warm() {
    use std::os::unix::fs::PermissionsExt as _;
    let root = workspace(&[("a.toml", "# one\n"), ("locked.toml", "# locked\n")]);
    std::fs::set_permissions(root.path().join("locked.toml"), std::fs::Permissions::from_mode(0o000)).expect("a mode");
    let locked_is_unreadable = std::fs::read(root.path().join("locked.toml")).is_err();
    let store = tempfile::tempdir().expect("a store");
    let mut asked = request(root.path(), vec![("idiomatic/inventory", rule(INVENTORY))]);
    let Corpus::Listed(listed) = &mut asked.corpus else { unreachable!("a listed corpus") };
    listed.push(ryo::linter::RelativePath("missing.toml".into()));
    let over_store = |request: LintRequest| {
        Linter::new(Languages(vec![ryo_toml::linked()]), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::At(store.path().to_path_buf()))
            .lint(vec![request])
            .pop()
            .expect("one answer")
            .expect("the lint is answered")
    };
    let cold = over_store(asked.clone());
    let warm = over_store(asked.clone());
    for (phase, report) in [("cold", &cold), ("warm", &warm)] {
        let rows = format!("{:?}", report.findings);
        assert!(rows.contains("a.toml"), "{phase}: the readable file has its row: {rows}");
        assert!(!rows.contains("missing.toml"), "{phase}: a listed file that is not there has no inventory row: {rows}");
        assert!(
            report.unknowns.iter().any(|unknown| format!("{unknown:?}").contains("missing.toml")),
            "{phase}: the relation is noted read only in part for the missing file: {:?}",
            report.unknowns
        );
        if locked_is_unreadable {
            assert!(rows.contains("locked.toml"), "{phase}: a file that is there and locked keeps its row: {rows}");
        }
    }
    assert_eq!(cold.findings, warm.findings, "a warm run answers what a cold one did");
    std::fs::set_permissions(root.path().join("locked.toml"), std::fs::Permissions::from_mode(0o644)).expect("a mode");
}

/// What a kept cache holds never replays a read failure: a file that could not be read and is later read, empty, answers as the empty
/// file does.
#[test]
fn a_file_that_could_not_be_read_and_is_later_read_empty_answers_as_the_empty_file_does() {
    let root = workspace(&[("a.toml", "# one\n")]);
    let store = tempfile::tempdir().expect("a store");
    let make = |with_the_file: bool| {
        let mut asked = request(root.path(), vec![("idiomatic/comments", rule(COMMENTS)), ("idiomatic/failures", rule(FAILURES))]);
        let Corpus::Listed(listed) = &mut asked.corpus else { unreachable!("a listed corpus") };
        if !with_the_file {
            listed.push(ryo::linter::RelativePath("late.toml".into()));
        } else if !listed.iter().any(|file| file.0.as_ref() == "late.toml") {
            listed.push(ryo::linter::RelativePath("late.toml".into()));
        }
        asked
    };
    let over_store = |request: LintRequest| {
        Linter::new(Languages(vec![ryo_toml::linked()]), WorkerPool::from(Workers(NonZeroUsize::MIN)), Cache::At(store.path().to_path_buf()))
            .lint(vec![request])
            .pop()
            .expect("one answer")
            .expect("the lint is answered")
    };
    let unreadable = over_store(make(false));
    assert!(format!("{:?}", unreadable.findings).contains("late.toml"), "the failure is a finding: {:?}", unreadable.findings);
    std::fs::write(root.path().join("late.toml"), "").expect("an empty file");
    let later = over_store(make(true));
    assert!(
        !format!("{:?}", later.findings).contains("late.toml"),
        "the file is read now: no read failure for it: {:?}",
        later.findings
    );
    assert!(
        !later.unread.iter().any(|unread| format!("{unread:?}").contains("late.toml")),
        "the file is not unread now: {:?}",
        later.unread
    );
}
