//! Relations the host declares beside the engine's own, read by a rule through `Linter::lint` (the old use-case test
//! `the_rows_of_a_relation_the_host_declared_are_read_by_a_rule`, re-pointed): a rule over host rows is a cross-file step, whose columns
//! are the host's.

use ryo::linter::{
    Cache, Cell, Corpus, HostColumn, HostColumnName, HostKind, HostRelation, HostRelationName, HostRow, HostText, Languages, LintReport,
    LintRequest, Linter, Rule, RuleApplicability, RuleId, SourceLanguage, Slot, WorkerPool, Workers,
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

const PLACED: &str = r#"let rows = select * from "host.placed";
let named = extend(from=rows, values={"problem": "placed"});
emit named subject "path" key("spec");"#;

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
        // The executor reads the files the host lists (a closed inventory): `Corpus::Walked` is refused by it .
        corpus: Corpus::Listed(listed(root)),
        beside: Vec::new(),
    }
}

fn placed() -> HostRelation {
    let row = |path: &str, spec: &str| HostRow(vec![Cell::File(ryo::linter::RelativePath(path.into())), Cell::Text(HostText(spec.into()))]);
    let column = |name: &str, kind| HostColumn { name: HostColumnName(name.into()), kind };
    HostRelation {
        name: HostRelationName("host.placed".into()),
        columns: vec![column("path", HostKind::File), column("spec", HostKind::Text)],
        rows: vec![row("a.toml", "docs"), row("not-there.toml", "docs")],
    }
}

fn run(files: &[(&str, &str)], hosted: Vec<HostRelation>) -> LintReport {
    let root = tempfile::tempdir().expect("a directory");
    for (path, text) in files {
        std::fs::write(root.path().join(path), text).expect("a file");
    }
    let mut asked = request(root.path(), vec![("idiomatic/placed", rule(PLACED))]);
    asked.hosted = hosted;
    linter().lint(vec![asked]).pop().expect("one answer").expect("answered")
}

/// Old: the rows of a relation the host declared are read by a rule: the row of a file the run lists is a finding; the row naming a file
/// the run does not list makes none.
#[test]
fn the_rows_of_a_relation_the_host_declared_are_read_by_a_rule() {
    let report = run(&[("a.toml", "# one\n"), ("b.toml", "# two\n")], vec![placed()]);
    assert_eq!(report.findings.len(), 1, "{report:?}");
    let finding = &report.findings[0];
    assert_eq!(format!("{:?}", finding.subject), "Is(File(RelativePath(\"a.toml\")))");
    let spec = finding.columns.iter().find(|column| format!("{:?}", column.name) == "ColumnName(\"spec\")").expect("the host's column");
    assert_eq!(spec.value, Slot::Is(Cell::Text(HostText("docs".into()))));
}

/// A host relation with no row answers nothing, without refusing the rule.
#[test]
fn a_host_relation_without_rows_answers_no_findings() {
    let mut empty = placed();
    empty.rows.clear();
    let report = run(&[("a.toml", "# one\n")], vec![empty]);
    assert!(report.findings.is_empty() && report.unread.is_empty(), "{report:?}");
}

/// The rule names a relation nothing declares: refused where the rules are admitted, before a file is read.
#[test]
fn a_relation_nobody_declared_is_refused() {
    let root = tempfile::tempdir().expect("a directory");
    std::fs::write(root.path().join("a.toml"), "# one\n").expect("a file");
    let error = linter()
        .lint(vec![request(root.path(), vec![("idiomatic/placed", rule(PLACED))])])
        .pop()
        .expect("one answer")
        .expect_err("refused");
    assert!(matches!(error, ryo::linter::LintError::Rule { .. }), "{error:?}");
}
