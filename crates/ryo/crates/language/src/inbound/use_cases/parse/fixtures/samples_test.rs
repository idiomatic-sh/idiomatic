//! A `harness = false` case answers libtest's command line, reports its verdict and keeps it: a pass returns, a failure
//! still fails, a listing names the case without running it, and a filter that leaves it out runs nothing.

use super::case_by_arguments;

/// A libtest command line of `words`.
fn line(words: &[&str]) -> Vec<String> {
    words.iter().map(|word| (*word).to_owned()).collect()
}

/// What the case `name` writes for the command line `words` when its body is `body`.
fn written(words: &[&str], name: &str, body: impl FnOnce()) -> String {
    let mut out = Vec::new();
    case_by_arguments(&line(words), name, &mut out, body);
    String::from_utf8(out).expect("the report is text")
}

#[test]
fn a_passing_case_returns() {
    assert_eq!(
        written(&[], "harness::passes", || {}),
        "\nrunning 1 test\ntest harness::passes ... ok\n\ntest result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out\n\n"
    );
}

#[test]
fn a_panicking_case_still_panics_after_its_report() {
    let outcome = std::panic::catch_unwind(|| {
        written(&[], "harness::fails", || panic!("the planted failure"))
    });
    assert!(
        outcome.is_err(),
        "a failed case resumes its panic after the report"
    );
}

#[test]
fn a_listing_names_the_case_without_running_it() {
    assert_eq!(
        written(&["--list", "--format", "terse"], "harness::listed", || {
            panic!("a listing runs no case")
        }),
        "harness::listed: test\n"
    );
    assert_eq!(
        written(&["--list"], "harness::listed", || panic!(
            "a listing runs no case"
        )),
        "harness::listed: test\n"
    );
}

#[test]
fn a_listing_of_the_ignored_cases_names_none() {
    assert_eq!(
        written(
            &["--list", "--format", "terse", "--ignored"],
            "harness::listed",
            || panic!("a listing runs no case")
        ),
        ""
    );
}

#[test]
fn a_filter_that_names_another_case_runs_nothing() {
    assert_eq!(
        written(
            &["harness::other", "--exact", "--nocapture"],
            "harness::filtered",
            || panic!("a case the filter leaves out is not run")
        ),
        ""
    );
}

#[test]
fn a_filter_that_names_the_case_runs_it() {
    let outcome = std::panic::catch_unwind(|| {
        written(
            &["harness::named", "--exact", "--nocapture"],
            "harness::named",
            || panic!("the case the filter names is run"),
        )
    });
    assert!(outcome.is_err(), "the case the filter names is run");
}
