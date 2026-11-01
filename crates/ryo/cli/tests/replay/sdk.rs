//! The public SDK over every provider this build links: each reads a comment of its language, and every name the catalog
//! lists is described under that name.

use std::path::Path;

/// The pool every run of these cases reads files on, built once for each run as a caller builds it.
fn pool() -> ryo_sdk::linter::WorkerPool {
    ryo_sdk::linter::WorkerPool::from(ryo_sdk::linter::Workers(
        std::num::NonZeroUsize::MIN.saturating_add(1),
    ))
}

/// The actual providers enabled by this CLI build, each paired with a comment in its language.
pub(crate) fn sdk_samples() -> Vec<(&'static str, &'static str, ryo_sdk::languages::Language)> {
    vec![
        #[cfg(feature = "css")]
        ("a.css", "/* café */\n", ryo_css::linked()),
        #[cfg(feature = "go")]
        ("a.go", "package a\n// café\n", ryo_go::linked()),
        #[cfg(feature = "html")]
        (
            "a.html",
            "<!doctype html><!-- café -->\n",
            ryo_html::linked(),
        ),
        #[cfg(feature = "javascript")]
        ("a.js", "// café\n", ryo_javascript::linked()),
        #[cfg(feature = "json")]
        ("a.jsonc", "// café\n", ryo_json::linked()),
        #[cfg(feature = "typescript")]
        ("a.ts", "// café\n", ryo_typescript::linked()),
        #[cfg(feature = "markdown")]
        ("a.md", "<!-- café -->\n", ryo_markdown::linked()),
        #[cfg(feature = "python")]
        ("a.py", "# café\n", ryo_python::linked()),
        #[cfg(feature = "rust")]
        ("a.rs", "// café\n", ryo_rust::linked()),
        #[cfg(feature = "toml")]
        ("a.toml", "# café\n", ryo_toml::linked()),
    ]
}

/// A public SDK request, independent of the CLI's core request types and adapters.
pub(crate) fn sdk_request(
    root: &Path,
    query: &str,
    sources: Vec<ryo_sdk::linter::CaseSource>,
) -> ryo_sdk::linter::LintRequest {
    use ryo_sdk::linter::{Corpus, LintRequest, Rule, RuleId};
    use ryo_sdk::query::RyoQlQuery;
    LintRequest {
        root: root.to_path_buf(),
        rules: std::collections::BTreeMap::from([(
            RuleId::try_from("consumer").expect("rule id"),
            Rule {
                query: RyoQlQuery::try_from(query).expect("query"),
                applicability: ryo_sdk::linter::RuleApplicability::Always,
                parameters: std::collections::BTreeMap::new(),
                include: Vec::new(),
                exclude: Vec::new(),
            },
        )]),
        files: Vec::new(),
        hosted: Vec::new(),
        corpus: Corpus::Given(sources),
        beside: Vec::new(),
    }
}

/// The providers selected independently by this build's feature flags.
fn sdk_provider_count() -> usize {
    [
        cfg!(feature = "css"),
        cfg!(feature = "go"),
        cfg!(feature = "html"),
        cfg!(feature = "javascript"),
        cfg!(feature = "json"),
        cfg!(feature = "typescript"),
        cfg!(feature = "markdown"),
        cfg!(feature = "python"),
        cfg!(feature = "rust"),
        cfg!(feature = "toml"),
    ]
    .into_iter()
    .filter(|enabled| *enabled)
    .count()
}

#[test]
fn public_sdk_reads_comments_with_every_enabled_provider() {
    use ryo_sdk::linter::{Cache, CaseSource, Languages, MemoryBudget, RelativePath, SourceText};
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("consumer project");
    let samples = sdk_samples();
    let expected = sdk_provider_count();
    assert_eq!(
        samples.len(),
        expected,
        "every enabled provider has an independent sample"
    );
    #[cfg(all(
        feature = "css",
        feature = "go",
        feature = "html",
        feature = "javascript",
        feature = "json",
        feature = "typescript",
        feature = "markdown",
        feature = "python",
        feature = "rust",
        feature = "toml"
    ))]
    assert_eq!(
        samples.len(),
        10,
        "the all-features gate exercises all ten languages"
    );
    let languages = Languages(
        samples
            .iter()
            .map(|(_, _, language)| language.clone())
            .collect(),
    );
    let sources = samples
        .iter()
        .map(|(path, text, _)| {
            std::fs::write(root.path().join(path), text).expect("source file");
            CaseSource {
                path: RelativePath((*path).into()),
                text: SourceText((*text).into()),
            }
        })
        .collect::<Vec<_>>();
    let comments = "let c = select * from \"comment\"; emit c subject file key(span);";
    let report = ryo_sdk::linter::lint(
        languages.clone(),
        &pool(),
        MemoryBudget::Machine,
        Cache::None,
        sdk_request(root.path(), comments, sources.clone()),
    )
    .expect("public lint");
    assert_eq!(report.findings.len(), expected);
    assert!(report.unread.is_empty());
    assert!(report.unknowns.is_empty());
    for finding in &report.findings {
        let text = finding
            .columns
            .iter()
            .find(|column| &*column.name.0 == "text")
            .expect("comment text column");
        let ryo_sdk::linter::Slot::Is(ryo_sdk::linter::Cell::Text(value)) = &text.value else {
            panic!("comment text is text, not {:?}", text.value);
        };
        assert_eq!(
            value.0.trim(),
            "café",
            "comment content for {:?}",
            finding.subject
        );
    }
    let absent = ryo_sdk::linter::lint(
        Languages::default(),
        &pool(),
        MemoryBudget::Machine,
        Cache::None,
        sdk_request(root.path(), comments, sources),
    )
    .expect("absent providers report unread");
    assert_eq!(absent.unread.len(), expected);
    sdk_query_counts_every_comment(&languages, root.path(), comments, expected);
    sdk_validate_refuses_a_misspelled_kind(&languages);
    sdk_catalog_describes(&languages);
}

/// A query over the same project counts one comment for each language, and shows only as many as its limit.
fn sdk_query_counts_every_comment(
    languages: &ryo_sdk::linter::Languages,
    root: &Path,
    comments: &str,
    expected: usize,
) {
    use ryo_sdk::linter::MemoryBudget;
    use ryo_sdk::query::{RowLimit, RunRequest, RyoQlQuery};
    let query = ryo_sdk::query::run(
        languages.clone(),
        &pool(),
        MemoryBudget::Machine,
        RunRequest {
            root: root.to_path_buf(),
            query: RyoQlQuery::try_from(comments).expect("query"),
            limit: RowLimit(1),
        },
    )
    .expect("public query");
    assert_eq!(
        query.total.0,
        u64::try_from(expected).expect("language count"),
        "the query counts one comment for each language"
    );
    assert_eq!(
        query.shown.0,
        u64::try_from(expected.min(1)).expect("shown count"),
        "and shows only as many as its limit"
    );
}

/// A kind no linked language has is refused by validate as a run refuses it.
fn sdk_validate_refuses_a_misspelled_kind(languages: &ryo_sdk::linter::Languages) {
    use ryo_sdk::query::RyoQlQuery;
    let misspelled = ryo_sdk::query::validate(
        languages,
        ryo_sdk::query::ValidateRequest {
            query: RyoQlQuery::try_from(
                r#"let n = select * from "syntax-tree" where kind = "nosuchkind"; emit n subject file key(span);"#,
            )
            .expect("query"),
            hosted: Vec::new(),
        },
    );
    assert!(
        matches!(
            misspelled,
            Err(ryo_sdk::query::ValidateError::Invalid { .. })
        ),
        "validate refuses a kind no language has: {misspelled:?}"
    );
}

/// Every catalog name is accepted by the public description endpoint with linked providers, which describes it under that name.
fn sdk_catalog_describes(languages: &ryo_sdk::linter::Languages) {
    for relation in ryo_sdk::relation::list()
        .expect("relation catalog")
        .relations
    {
        let request = ryo_sdk::relation::DescribeRequest {
            name: relation.name.clone(),
        };
        let described = ryo_sdk::relation::describe(languages.clone(), request)
            .expect("listed relation describes through public SDK");
        assert_eq!(
            described.relation.name, relation.name,
            "described as listed"
        );
    }
    for operator in ryo_sdk::operator::list()
        .expect("operator catalog")
        .operators
    {
        let request = ryo_sdk::operator::DescribeRequest {
            name: operator.name.clone(),
        };
        let described = ryo_sdk::operator::describe(request)
            .expect("listed operator describes through public SDK");
        assert_eq!(
            described.operator.name, operator.name,
            "described as listed"
        );
    }
}

/// A Cargo workspace of two packages, one depending on the other by path and using it from its source.
#[cfg(feature = "rust")]
const WORKSPACE: &[(&str, &str)] = &[
    ("Cargo.toml", "[workspace]\nmembers = [\"a\", \"b\"]\n"),
    (
        "a/Cargo.toml",
        "[package]\nname = \"a\"\nversion = \"0.1.0\"\n",
    ),
    ("a/src/lib.rs", "pub fn a() {}\n"),
    (
        "b/Cargo.toml",
        "[package]\nname = \"b\"\nversion = \"0.1.0\"\n\n[dependencies]\na = { path = \"../a\" }\n",
    ),
    ("b/src/lib.rs", "mod inner;\npub fn b() { a::a(); }\n"),
    ("b/src/inner.rs", "pub fn inner() { crate::b(); }\n"),
];

#[cfg(feature = "rust")]
#[test]
fn files_given_as_text_fill_the_project_relations_as_the_same_files_walked_on_disk() {
    use ryo_sdk::linter::{
        Cache, CaseSource, Corpus, Languages, MemoryBudget, RelativePath, RuleId, SourceText,
    };
    use ryo_sdk::query::RyoQlQuery;
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("a directory");
    for (path, text) in WORKSPACE {
        let place = root.path().join(path);
        std::fs::create_dir_all(place.parent().expect("a parent")).expect("a directory");
        std::fs::write(place, text).expect("a file");
    }
    let languages = Languages(vec![ryo_rust::linked()]);
    let sources: Vec<CaseSource> = WORKSPACE
        .iter()
        .map(|(path, text)| CaseSource {
            path: RelativePath((*path).into()),
            text: SourceText((*text).into()),
        })
        .collect();
    let functions = "let f = select * from \"function\"; emit f subject file key(span);";
    let mut given = sdk_request(Path::new("/nowhere/given"), functions, sources);
    for (id, query) in [
        (
            "members",
            "let m = select * from \"workspace-member\"; emit m subject package key(package);",
        ),
        (
            "dependencies",
            "let d = select * from \"package-dependency\"; emit d subject source key(package);",
        ),
        (
            "sources",
            "let d = select * from \"source-dependency\"; emit d subject source key(target);",
        ),
    ] {
        let mut rule = given.rules.values().next().expect("a rule").clone();
        rule.query = RyoQlQuery::try_from(query).expect("query");
        assert!(
            given
                .rules
                .insert(RuleId::try_from(id).expect("rule id"), rule)
                .is_none()
        );
    }
    let mut walked = given.clone();
    walked.root = root.path().to_path_buf();
    walked.corpus = Corpus::Walked;
    let findings = |request| {
        let mut shown: Vec<String> = ryo_sdk::linter::lint(
            languages.clone(),
            &pool(),
            MemoryBudget::Machine,
            Cache::None,
            request,
        )
        .expect("the rules run")
        .findings
        .iter()
        .map(|finding| format!("{finding:?}"))
        .collect();
        shown.sort();
        shown
    };
    let on_disk = findings(walked);
    for relation in ["members", "dependencies", "sources"] {
        assert!(
            on_disk
                .iter()
                .any(|each| each.contains(&format!("\"{relation}\""))),
            "the walked run finds rows of {relation}: {on_disk:#?}"
        );
    }
    assert_eq!(findings(given), on_disk);
}
