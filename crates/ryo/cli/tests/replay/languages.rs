//! The replay links what `ryo lint` links (`ryo_cli::linked_connections`): a rule may name a kind of any linked language, and over
//! files of another language that kind matches nothing.

use std::sync::Arc;

#[test]
fn a_rule_naming_a_kind_of_another_linked_language_loads_and_answers() {
    let _beside = crate::beside();
    // Here a language beside Rust and TOML, whose kinds a rule may name.
    assert!(
        ryo_cli::linked_connections().len() > 2,
        "the command links no language beside Rust and TOML"
    );
    // `keyword` is a kind of the Python and TypeScript languages, not of Rust's: over Rust files it matches nothing.
    let query = r#"let words = select l."file" as "file", l."span" as "span" from "syntax-tree" as l where l."kind" = "keyword";
emit words subject "file" key("span");"#;
    let rule = crate::tools::rule_from_query("keywords", query, &["**".to_owned()]);
    let repository = tempfile::tempdir().expect("a repository");
    std::fs::write(repository.path().join("lib.rs"), "fn alpha() {}\n").expect("its lib.rs");
    let (rows, _) = crate::base::lint_report(
        repository.path(),
        vec![rule],
        Arc::new(ryo_execution_runtime::outbound::facts::factory::memory()),
        Arc::new(ryo_core::outbound::lint_store::fixtures::fake::LintStoreFake::default()),
    )
    .expect("a rule naming a kind of a linked language runs");
    assert_eq!(rows, Vec::<Vec<String>>::new());
}
