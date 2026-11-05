//! What the project writes from the manifests, `workspace-member` and `package-dependency`, is read by the steps that
//! run after the files: a package a workspace holds that the run's scope does not list is found. What those rows say is held to
//! what each ecosystem's own tool says of the same project in the `manifests_<tool>` suites, which share `tools`.

use crate::base;
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::types::Rule;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::runs::types::RuleId;

/// A workspace of two packages.
pub(crate) fn workspace() -> tempfile::TempDir {
    let root = tempfile::tempdir().expect("a directory");
    let write = |path: &str, text: &str| {
        let at = root.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, text).expect("a file");
    };
    write(
        "Cargo.toml",
        "[workspace]\nresolver = \"2\"\nmembers = [\"inside\", \"outside\"]\n",
    );
    for package in ["inside", "outside"] {
        write(
            &format!("{package}/Cargo.toml"),
            &format!("[package]\nname = \"{package}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n"),
        );
        write(&format!("{package}/src/lib.rs"), "pub fn run() {}\n");
    }
    root
}

#[test]
fn a_member_of_a_workspace_the_scope_does_not_list_is_found() {
    let _beside = crate::beside();
    let root = workspace();
    let rule = Rule {
        observations: Vec::new(),
        id: RuleId("members".into()),
        query: ryo_values::domain::program::types::QueryBundle::from(QueryText("let members = select * from \"workspace-member\"; emit members subject package key(package);".into())),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        parameters: Vec::new(),
        include: vec![Glob("**".into())],
        exclude: vec![Glob("outside/**".into())],
    };
    let (rows, _) = base::lint_with(
        root.path(),
        vec![rule],
        Arc::new(ryo_core::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rule runs");
    assert!(!rows.is_empty(), "the rule gives a row for each member");
}

#[test]
fn a_package_the_scope_does_not_list_is_found_by_the_recorded_workspace_completeness_query() {
    let _beside = crate::beside();
    let root = workspace();
    let query = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/recordings/workspace-completeness/query.ryoql"
    ))
    .expect("the recorded query");
    let mut rule = crate::rule_from_query("workspace-completeness", &query, &["**".to_owned()]);
    rule.exclude = vec![Glob("outside/**".into())];
    let (rows, _) = base::lint_with(
        root.path(),
        vec![rule],
        Arc::new(ryo_core::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rule runs");
    assert_eq!(
        rows,
        vec![vec![
            "workspace-completeness".to_owned(),
            "Cargo.toml".to_owned(),
            "Cargo.toml,outside/Cargo.toml".to_owned(),
            String::new(),
            String::new()
        ]]
    );
}

/// A package the listing does not hold at all, which a workspace names, is a file all the same: the rows of the project give it a number
/// after the files the listing holds, and a rule that compares it with the files finds it.
#[test]
fn a_member_the_listing_does_not_hold_is_found_as_a_file_after_the_listed_ones() {
    let _beside = crate::beside();
    let root = workspace();
    std::fs::write(root.path().join(".ignore"), "outside/\n").expect("an ignore file");
    let query = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/tests/fixtures/recordings/workspace-completeness/query.ryoql"
    ))
    .expect("the recorded query");
    let rule = crate::rule_from_query("workspace-completeness", &query, &["**".to_owned()]);
    let (rows, _) = base::lint_with(
        root.path(),
        vec![rule],
        Arc::new(ryo_core::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("the rule runs");
    assert_eq!(
        rows,
        vec![vec![
            "workspace-completeness".to_owned(),
            "Cargo.toml".to_owned(),
            "Cargo.toml,outside/Cargo.toml".to_owned(),
            String::new(),
            String::new()
        ]]
    );
}

#[test]
fn a_manifest_that_does_not_parse_is_a_finding_and_the_others_are_read_cold_and_warm() {
    let _beside = crate::beside();
    let root = workspace();
    let at = root.path().join("broken/Cargo.toml");
    std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
    std::fs::write(&at, "[package\nbroken").expect("a file");
    let rule = Rule {
        observations: Vec::new(),
        id: RuleId("members".into()),
        query: ryo_values::domain::program::types::QueryBundle::from(QueryText("let members = select * from \"workspace-member\"; emit members subject package key(package);".into())),
        applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
        parameters: Vec::new(),
        include: vec![Glob("**".into())],
        exclude: Vec::new(),
    };
    let kept = || base::store_at(root.path());
    let cold = crate::counting::lint_counting(root.path(), vec![rule.clone()], kept())
        .expect("a manifest that does not parse stops nothing");
    assert!(
        cold.unread.iter().any(|path| path == "broken/Cargo.toml"),
        "the refused manifest is reported: {:?}",
        cold.unread
    );
    assert!(
        !cold.rows.is_empty(),
        "the manifests that parse are still read"
    );
    let warm = crate::counting::lint_counting(root.path(), vec![rule], kept()).expect("a warm run");
    assert_eq!(warm.unread, cold.unread, "a warm run reports the same");
    assert_eq!(warm.rows, cold.rows);
}

/// A workspace of one package, in a directory beside a Cargo configuration of the directory above it.
fn beside_a_configuration() -> tempfile::TempDir {
    let above = tempfile::tempdir().expect("a directory");
    let write = |path: &str, text: &str| {
        let at = above.path().join(path);
        std::fs::create_dir_all(at.parent().expect("a parent")).expect("a directory");
        std::fs::write(at, text).expect("a file");
    };
    write(
        ".cargo/config.toml",
        "[build]\ntarget-dir = \"repository/out\"\n",
    );
    write(
        "repository/Cargo.toml",
        "[workspace]\nmembers = [\"inside\"]\n",
    );
    write(
        "repository/inside/Cargo.toml",
        "[package]\nname = \"inside\"\nversion = \"0.1.0\"\n",
    );
    write("repository/inside/src/lib.rs", "pub fn run() {}\n");
    above
}

#[test]
fn a_change_to_a_configuration_above_the_repository_reads_the_project_again() {
    let _beside = crate::beside();
    let above = beside_a_configuration();
    let root = above.path().join("repository");
    let rule = crate::rule_from_query(
        "members",
        "let members = select * from \"workspace-member\"; emit members subject package key(package);",
        &["**".to_owned()],
    );
    let kept: Arc<ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake> = Arc::default();
    let run = || {
        crate::counting::lint_counting(
            &root,
            vec![rule.clone()],
            Arc::<ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake>::clone(&kept),
        )
        .expect("the rule runs")
    };
    let cold = run();
    assert_eq!(cold.projects, 1, "the project is read once the first time");
    let warm = run();
    assert_eq!(
        warm.projects, 0,
        "and not by the next run, when nothing it read changed"
    );
    assert_eq!(warm.rows, cold.rows);
    std::fs::write(
        above.path().join(".cargo/config.toml"),
        "[build]\ntarget-dir = \"repository/other\"\n",
    )
    .expect("a file");
    let changed = run();
    assert_eq!(
        changed.projects, 1,
        "and again when the configuration above it changed"
    );
    assert_eq!(changed.rows, cold.rows);
}
