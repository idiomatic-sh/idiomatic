//! A damaged input is reported, never the abort of the run.

/// A TOML file written in UTF-16 is the report of its syntax errors, never the abort of the run: the query over the project answers.
#[cfg(feature = "toml")]
#[test]
fn a_utf16_toml_file_does_not_abort_the_run() {
    use ryo_sdk::linter::{Languages, MemoryBudget};
    use ryo_sdk::query::{RowLimit, RunRequest, RyoQlQuery};
    let _beside = crate::beside();
    let root = tempfile::tempdir().expect("consumer project");
    std::fs::write(
        root.path().join("utf16-comment.toml"),
        include_bytes!(
            "../../../languages/toml/tests/fixtures/suite/invalid/encoding/utf16-comment.toml"
        ),
    )
    .expect("the fixture");
    std::fs::write(root.path().join("ok.toml"), "# fine\na = 1\n").expect("a file");
    let answer = ryo_sdk::query::run(
        Languages(vec![ryo_toml::linked()]),
        &ryo_sdk::linter::WorkerPool::from(ryo_sdk::linter::Workers(
            std::num::NonZeroUsize::MIN.saturating_add(1),
        )),
        MemoryBudget::Machine,
        RunRequest {
            root: root.path().to_path_buf(),
            queries: vec![RyoQlQuery::try_from(
                r#"let e = select * from "syntax-error"; emit e subject file key(span);"#,
            )
            .expect("query")],
            limit: RowLimit(10),
        },
    )
    .expect("the run answers, it does not abort");
    assert!(
        answer.total.0 >= 1,
        "the file that does not parse gives its syntax errors: {answer:?}"
    );
}
