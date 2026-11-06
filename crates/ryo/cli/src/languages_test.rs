//! Proves the skips of the languages this build links are what the walks read: the languages' own declarations, not a list a test hands in.

use ryo_core::inbound::use_cases::lint::interface::LintUseCase as _;
use ryo_core::inbound::use_cases::lint::types::{LintRequest, Rule};
use ryo_language::inbound::use_cases::project::types::RootPath;
use ryo_values::domain::plan_parts::types::QueryText;
use ryo_values::domain::row_texts::types::RowTotal;
use ryo_values::domain::runs::types::AnswerShape;
use ryo_values::domain::runs::types::RuleId;
use ryo_values::domain::runs::types::{Budget, Session};

/// Dependency, environment and build directories at the root and nested, and two files in the source directory; no language reads a `.txt`
/// file, so each one a run walks is listed unread.
const FILES: [&str; 7] = [
    "node_modules/x.txt",
    "pkg/node_modules/v.txt",
    ".venv/lib/y.txt",
    "target/z.txt",
    "pkg/target/w.txt",
    "src/a.txt",
    "src/b.txt",
];

/// `query run`, composed as the command composes it, does not go into the directories the linked languages declare.
#[test]
fn the_walk_does_not_enter_the_directories_the_linked_languages_declare() {
    let directory = tempfile::tempdir().expect("a temporary directory");
    for file in FILES {
        let path = directory.path().join(file);
        std::fs::create_dir_all(path.parent().expect("a directory")).expect("a directory");
        std::fs::write(path, "x").expect("a file");
    }
    let report = crate::resources::query::run::composition::build(
        Box::new(std::io::sink()),
        &crate::resources::edge_at(
            RootPath(".".into()),
            RootPath(".".into()),
            crate::output::Format::Terminal,
        ),
        ryo_core::outbound::lint_store::types::Store::default(),
        RootPath(directory.path().to_str().expect("a utf-8 path").into()),
        Budget::Machine,
        ryo_values::domain::runs::types::WorkersAsked(0),
    )
    .lint(LintRequest {
        source_language: None,
        session: Session {
            search_scope: Default::default(),
            given_inventory: Default::default(),
            repository: ryo_language::inbound::use_cases::project::types::Repository::Disk,
            root: RootPath(directory.path().to_str().expect("a utf-8 path").into()),
        },
        files: Vec::new(),
        rules: vec![Rule {
            observations: Vec::new(),
            id: RuleId("1".into()),
            query: ryo_values::domain::query_tokens::types::QueryBundle::from(QueryText(
                "let functions = select * from \"function\"; emit functions subject \"file\" key(\"span\");".into()),
            ),
            applicability: ryo_values::domain::runs::types::RuleApplicability::Always,
            parameters: Vec::new(),
            include: Vec::new(),
            exclude: Vec::new(),
        }],
        hosted: Vec::new(),
        answer: AnswerShape::Rows(RowTotal(20)),
    })
    .expect("a run");
    let mut found = report
        .unread
        .iter()
        .map(|file| file.path.0.to_string())
        .collect::<Vec<_>>();
    found.sort();
    assert_eq!(found, ["src/a.txt", "src/b.txt"]);
}

/// The linked set declares the directories of the languages it links.
#[test]
fn the_linked_languages_declare_their_directories() {
    let declared = crate::languages::skipped().0;
    for name in ["node_modules", ".venv", "target"] {
        assert!(declared.iter().any(|skip| &**skip == name), "{name}");
    }
}
