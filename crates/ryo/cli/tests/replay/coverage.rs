//! A relation a file is read in part makes the rules that read it say what they could not decide, and the steps that answer from the
//! absence of its rows make none where those rows could be missing.

use crate::base;
use std::path::Path;
use std::sync::Arc;

/// Rows of a rule by file, or what a rule could not decide in a file: the rule and the path.
type Pairs = Vec<(String, String)>;

/// What the rules could not decide: the rule, the path and why.
type Undecided = Vec<(String, String, String)>;

/// A directory of the files given.
fn directory(files: &[(&str, &str)]) -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    for (path, text) in files {
        std::fs::write(root.path().join(path), text).expect("a file");
    }
    root
}

/// The variables a file defines, as a rule that only selects.
const SELECTS: &str = "let variables = select * from \"defines\" where kind = \"variable\"; emit variables subject \"file\" key(\"span\");";

/// The variables no definition of the same file has the name `nothing` for: an antijoin in the file, which trusts there is none.
const ANTIJOINS: &str = "let variables = select * from \"defines\" where kind = \"variable\"; let nothing = select * from \"defines\" where name = \"nothing\"; let alone = antijoin(left=variables, right=nothing, left-on=[\"file\"], right-on=[\"file\"]); emit alone subject \"file\" key(\"span\");";

/// How many variables of each name there are across the files: a count the steps after the files make.
const COUNTS: &str = "let variables = select * from \"defines\" where kind = \"variable\"; let counted = group-count(from=variables, keys=[\"name\"], count=\"uses\"); let several = select * from counted where uses >= 1; emit several subject \"name\" key(\"name\");";

/// The files lint with the three rules: the rows of each rule by file, and what could not be decided.
fn lint(files: &[(&str, &str)]) -> (Pairs, Pairs) {
    let _beside = crate::beside();
    let root = directory(files);
    let rules = [
        ("selects", SELECTS),
        ("antijoins", ANTIJOINS),
        ("counts", COUNTS),
    ]
    .iter()
    .map(|(id, query)| crate::rule_from_query(id, query, &["**".to_owned()]))
    .collect();
    let (rows, report) = base::lint_report(
        root.path(),
        rules,
        Arc::new(ryo_core::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rules run");
    let mut rows: Vec<(String, String)> = rows
        .into_iter()
        .filter_map(|row| Some((row.first()?.clone(), row.get(1)?.clone())))
        .collect();
    rows.sort();
    let mut unknowns: Vec<(String, String)> = report
        .unknowns
        .into_iter()
        .map(|each| (each.rule.0.to_string(), each.path.0.to_string()))
        .collect();
    unknowns.sort();
    (rows, unknowns)
}

fn pair(rule: &str, path: &str) -> (String, String) {
    (rule.to_owned(), path.to_owned())
}

/// A file with a single-name `let` and a destructuring one, and a file with a single-name `let`.
const BOTH: [(&str, &str); 2] = [
    (
        "a.rs",
        "fn a() {\n    let w = 1;\n    let (Unit, y) = (1, 2);\n}\n",
    ),
    ("b.rs", "fn b() {\n    let z = 1;\n}\n"),
];

#[test]
fn files_read_whole_give_their_rows_and_decide_everything() {
    let (rows, unknowns) = lint(&[("a.rs", "fn a() {\n    let w = 1;\n}\n"), BOTH[1]]);
    assert_eq!(
        rows,
        vec![
            pair("antijoins", "a.rs"),
            pair("antijoins", "b.rs"),
            pair("counts", "w"),
            pair("counts", "z"),
            pair("selects", "a.rs"),
            pair("selects", "b.rs"),
        ]
    );
    assert_eq!(unknowns, Vec::new());
}

#[test]
fn a_file_with_a_name_in_a_pattern_that_may_be_a_constant_makes_the_rules_that_read_defines_say_so()
{
    let (_, unknowns) = lint(&BOTH);
    assert_eq!(
        unknowns,
        vec![
            pair("antijoins", "a.rs"),
            pair("counts", "a.rs"),
            pair("selects", "a.rs"),
        ]
    );
}

#[test]
fn rows_the_absence_of_rows_would_make_are_dropped_where_rows_could_be_missing_and_found_rows_stay()
{
    let (rows, _) = lint(&BOTH);
    // The select keeps what it found in both files; the antijoin of the file keeps what its whole file proves; the count by name,
    // which any file could change, makes no row where a file is read in part.
    assert_eq!(
        rows,
        vec![
            pair("antijoins", "b.rs"),
            pair("selects", "a.rs"),
            pair("selects", "a.rs"),
            pair("selects", "b.rs"),
        ]
    );
}

#[test]
fn a_file_with_syntax_errors_is_read_in_part_in_every_relation_of_code() {
    let (rows, unknowns) = lint(&[("a.rs", "fn a( {\n    let w = 1;\n}\n")]);
    assert_eq!(
        unknowns,
        vec![
            pair("antijoins", "a.rs"),
            pair("counts", "a.rs"),
            pair("selects", "a.rs"),
        ]
    );
    assert!(rows.iter().all(|row| row.0 == "selects"));
}

/// Lints the directory with the three rules, with the store given, and shows what it gave.
fn lint_in(
    root: &Path,
    kept: Arc<ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake>,
) -> (Vec<Vec<String>>, Undecided) {
    let rules = [
        ("selects", SELECTS),
        ("antijoins", ANTIJOINS),
        ("counts", COUNTS),
    ]
    .iter()
    .map(|(id, query)| crate::rule_from_query(id, query, &["**".to_owned()]))
    .collect();
    let (mut rows, report) = base::lint_report(
        root,
        rules,
        Arc::new(ryo_core::outbound::facts::factory::memory()),
        kept,
    )
    .expect("the rules run");
    rows.sort();
    let unknowns = report
        .unknowns
        .into_iter()
        .map(|each| {
            (
                each.rule.0.to_string(),
                each.path.0.to_string(),
                each.message.0.to_string(),
            )
        })
        .collect();
    (rows, unknowns)
}

#[test]
fn what_a_file_was_read_in_part_is_kept_with_the_file_and_gives_the_same_answer_warm_and_after_an_edit()
 {
    let _beside = crate::beside();
    let root = directory(&BOTH);
    let kept = Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default());
    let cold = lint_in(root.path(), Arc::clone(&kept));
    assert!(!cold.1.is_empty());
    assert_eq!(lint_in(root.path(), Arc::clone(&kept)), cold, "warm");
    // Another file edited: the file read in part is not read again, and what it costs the rules is as it was.
    std::fs::write(root.path().join("b.rs"), "fn b() {\n    let q = 1;\n}\n").expect("a file");
    let edited = lint_in(root.path(), Arc::clone(&kept));
    let fresh = lint_in(
        root.path(),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    );
    assert_eq!(edited, fresh, "after an edit");
    // The edit that makes the file whole again takes its unknowns away.
    std::fs::write(root.path().join("a.rs"), "fn a() {\n    let w = 1;\n}\n").expect("a file");
    assert!(lint_in(root.path(), kept).1.is_empty());
}

/// The reasons the linked languages declare for a value they cannot resolve are numbered apart: one number is one name in every
/// language, so a shown unknown value names its reason whichever language wrote it.
#[test]
fn the_unknown_reasons_of_the_linked_languages_are_numbered_apart() {
    let declared: Vec<(u8, &str)> = crate::sdk::sdk_samples()
        .iter()
        .flat_map(|(_, _, language)| language.parse_engine().unknown_reasons().to_vec())
        .map(|name| (name.reason.0, name.text.0))
        .collect();
    for (number, text) in &declared {
        assert!(
            declared
                .iter()
                .filter(|(other, _)| other == number)
                .all(|(_, same)| same == text),
            "reason {number} is `{text}` in one language and another thing in another"
        );
    }
}

/// Optional semantic facts are unavailable from Rust, rather than a known empty set.
const OPTIONAL_CAPABILITY_RULES: [(&str, &str); 8] = [
    (
        "shape-positive",
        "let rows = select * from \"expression-shape\"; emit rows subject file key(span);",
    ),
    (
        "shape-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"expression-shape\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
const OPTIONAL_CAPABILITY_RULES: [(&str, &str); 16] = [
    (
        "identifier-positive",
        "let rows = select * from \"identifier\"; emit rows subject file key(span);",
    ),
    (
        "identifier-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"identifier\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
    (
        "qualified-positive",
        "let rows = select * from \"qualified-name\"; emit rows subject file key(span);",
    ),
    (
        "qualified-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"qualified-name\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
    (
        "embedded-positive",
        "let rows = select * from \"embedded-expression\"; emit rows subject file key(span);",
    ),
    (
        "embedded-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"embedded-expression\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
    (
        "binding-event-positive",
        "let variables = select * from \"defines\" where kind = \"variable\"; let rows = select l.id as id, l.file as file, l.span as span from variables as l join \"binding-event\" as r on l.file = r.file; emit rows subject file key(span);",
    ),
    (
        "binding-event-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"binding-event\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
    (
        "binding-read-positive",
        "let variables = select * from \"defines\" where kind = \"variable\"; let rows = select l.id as id, l.file as file, l.span as span from variables as l join \"binding-read\" as r on l.file = r.file; emit rows subject file key(span);",
    ),
    (
        "binding-read-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"binding-read\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
    (
        "binding-use-positive",
        "let variables = select * from \"defines\" where kind = \"variable\"; let rows = select l.id as id, l.file as file, l.span as span from variables as l join \"binding-use\" as r on l.file = r.file; emit rows subject file key(span);",
    ),
    (
        "binding-use-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"binding-use\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
    (
        "binding-shadow-positive",
        "let variables = select * from \"defines\" where kind = \"variable\"; let rows = select l.id as id, l.file as file, l.span as span from variables as l join \"binding-shadow\" as r on l.file = r.file; emit rows subject file key(span);",
    ),
    (
        "binding-shadow-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"binding-shadow\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
    (
        "implicit-symbol-positive",
        "let variables = select * from \"defines\" where kind = \"variable\"; let rows = select l.id as id, l.file as file, l.span as span from variables as l join \"implicit-symbol\" as r on l.file = r.file; emit rows subject file key(span);",
    ),
    (
        "implicit-symbol-negative",
        "let variables = select * from \"defines\" where kind = \"variable\"; let absent = antijoin(left=variables, right=\"implicit-symbol\", left-on=[\"file\"], right-on=[\"file\"]); emit absent subject file key(span);",
    ),
];

#[test]
fn unadvertised_semantics_are_unknown_and_never_prove_absence_cold_or_warm() {
    let _beside = crate::beside();
    let root = directory(&[("input.rs", "fn example() { let value = 1; }")]);
    let kept = Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default());
    for phase in ["cold", "warm"] {
        let rules = OPTIONAL_CAPABILITY_RULES
            .iter()
            .chain(std::iter::once(&("witness", SELECTS)))
            .map(|(id, query)| crate::rule_from_query(id, query, &["**".to_owned()]))
            .collect();
        let (rows, report) = base::lint_report(
            root.path(),
            rules,
            Arc::new(ryo_core::outbound::facts::factory::memory()),
            kept.clone(),
        )
        .expect("optional capability queries compile and execute");
        assert!(
            report.unread.is_empty(),
            "{phase}: every input must be read"
        );
        assert_eq!(
            rows.len(),
            1,
            "{phase}: only the known variable witness is emitted: {rows:?}"
        );
        assert_eq!(
            rows.first().and_then(|row| row.first()).map(String::as_str),
            Some("witness")
        );
        let mut actual: Vec<_> = report
            .unknowns
            .iter()
            .map(|unknown| (unknown.rule.0.to_string(), unknown.path.0.to_string()))
            .collect();
        actual.sort();
        let mut expected: Vec<_> = OPTIONAL_CAPABILITY_RULES
            .iter()
            .map(|(rule, _)| pair(rule, "input.rs"))
            .collect();
        expected.sort();
        assert_eq!(
            actual, expected,
            "{phase}: every optional query must fail closed"
        );
        assert!(
            report
                .unknowns
                .iter()
                .all(|unknown| unknown.message.0.contains(
                    "The requested optional capability is unavailable from this provider."
                )),
            "{phase}: the unknown must explain the unavailable capability: {:?}",
            report.unknowns
        );
    }
}
