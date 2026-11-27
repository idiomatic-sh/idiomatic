//! The replay links what `ryo lint` links (`ryo_cli::linked_sdk`): a rule may name a kind of any linked language, and over
//! files of another language that kind matches nothing.

#[test]
fn a_rule_naming_a_kind_of_another_linked_language_loads_and_answers() {
    let _beside = crate::beside();
    // Here a language beside Rust and TOML, whose kinds a rule may name.
    assert!(
        ryo_cli::linked_sdk().0.len() > 2,
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
        ryo_sdk::linter::Cache::None,
    )
    .expect("a rule naming a kind of a linked language runs");
    assert_eq!(rows, Vec::<Vec<String>>::new());
}

/// A file whose extension no language reads, listed under a request that names a language, is read as that language, whichever it is;
/// listed under a request that names none, it is unread: the extension decides only when the request names no language.
#[test]
fn a_file_with_a_foreign_extension_is_read_as_the_language_the_request_names_for_every_linked_language() {
    use ryo_sdk::linter::{
        Cache, Corpus, Languages, LintRequest, Linter, Rule, RuleApplicability, RuleId, SourceLanguage, Unreadable, WorkerPool, Workers,
    };
    let _beside = crate::beside();
    let linked = ryo_cli::linked_sdk().0;
    assert!(linked.len() > 2, "the command links no language beside Rust and TOML");
    let query = ryo_sdk::query::RyoQlQuery::try_from(
        r#"let nodes = select * from "syntax-tree"; emit nodes subject file key(id);"#,
    )
    .expect("a query");
    for language in linked {
        let name = language.name().clone();
        let label = name.0.to_string();
        let root = tempfile::tempdir().expect("a directory");
        std::fs::write(root.path().join("input.foreign"), "").expect("a file");
        let lint = |source_language: SourceLanguage| {
            let linter = Linter::new(
                Languages(vec![language.clone()]),
                WorkerPool::from(Workers(std::num::NonZeroUsize::MIN)),
                Cache::None,
            );
            let request = LintRequest {
                source_language,
                root: root.path().to_path_buf(),
                project_search_scope: Default::default(),
                rules: [(
                    RuleId::try_from("nodes").expect("an id"),
                    Rule {
                        observations: Vec::new(),
                        query: query.clone(),
                        applicability: RuleApplicability::Always,
                        parameters: Default::default(),
                        include: Vec::new(),
                        exclude: Vec::new(),
                    },
                )]
                .into(),
                files: Vec::new(),
                hosted: Vec::new(),
                corpus: Corpus::Listed(vec![ryo_sdk::linter::RelativePath("input.foreign".into())]),
                beside: Vec::new(),
            };
            linter.lint(vec![request]).pop().expect("one answer")
        };
        let named = lint(SourceLanguage::Named(name.clone())).unwrap_or_else(|error| panic!("{label}: named, the file is read: {error}"));
        assert!(
            !named.unread.iter().any(|unread| matches!(unread.reason, Unreadable::NoLanguage)),
            "{label}: the file is read as the named language, not unread for lack of one: {:?}",
            named.unread
        );
        let unnamed = lint(SourceLanguage::Recognized).unwrap_or_else(|error| panic!("{label}: unnamed: {error}"));
        assert!(
            unnamed.unread.iter().any(|unread| matches!(unread.reason, Unreadable::NoLanguage)),
            "{label}: with no language named the extension decides, and no language reads it: {:?}",
            unnamed.unread
        );
    }
}
