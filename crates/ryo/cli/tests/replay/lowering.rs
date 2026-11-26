//! The rows of HTML's DOM order, its scalar values and its missing locations are the same on every run. Engine determinism over
//! owned rules is proved by the SDK (`a_shared_linter_gives_what_fresh_linters_give`), not over a corpus here.
#![cfg(feature = "html")]


/// Native DOM order, scalar values and missing locations must survive every run.
#[test]
fn html_dom_and_absent_locations_are_stable_across_runs() -> Result<(), Box<dyn std::error::Error>>
{
    let _alone = crate::alone();
    let directory = tempfile::tempdir()?;
    std::fs::write(
        directory.path().join("page.html"),
        "<!doctype html><table>foster<tr><td>&amp;</td></tr></table><p id='a'>end</p>",
    )?;
    let queries = [
        (
            "tree",
            r#"let rows = select * from "syntax-tree"; emit rows subject file key(id,text,span);"#,
        ),
        (
            "spans",
            r#"let rows = select l."id" as "left", r."id" as "right", l."file" as "file", l."span" as "span" from "syntax-tree" as l join "syntax-tree" as r on l."span" = r."span"; emit rows subject file key(left,right,span);"#,
        ),
    ];
    let mut answers = Vec::new();
    for _run in 0..2 {
        let html = ryo_html::linked();
        let result = crate::base::lint_report_with_languages(
            directory.path(),
            queries
                .iter()
                .map(|(name, query)| {
                    crate::tools::rule_from_query(name, query, &["**/*.html".to_owned()])
                })
                .collect(),
            ryo_sdk::linter::Cache::None,
            Vec::new(),
            false,
            vec![html],
        );
        let (mut rows, report) = result?;
        assert!(report.unread.is_empty(), "HTML is actually read");
        rows.sort();
        assert!(rows.iter().flatten().any(|cell| cell.contains("unlocated")));
        assert!(
            rows.iter()
                .flatten()
                .any(|cell| cell.contains("http://www.w3.org/1999/xhtml"))
        );
        answers.push(rows);
    }
    assert_eq!(answers[0], answers[1]);
    Ok(())
}
