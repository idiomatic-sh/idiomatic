//! The product README's Usage, each command run and its output compared.

/// The README's commands run as a new project's would: one after another in a directory of their own, empty but for
/// the git repository Tastemaker reads a workspace in. trycmd runs a Markdown file's blocks in a sandbox when an `.out`
/// directory sits beside it, starting from the `.in` directory beside it, so the README is copied beside both.
#[test]
fn readme() {
    let copy = tempfile::tempdir().expect("a directory for the README's copy");
    let readme = copy.path().join("README.md");
    let _: u64 = std::fs::copy("../README.md", &readme).expect("the README is copied");
    std::fs::create_dir_all(copy.path().join("README.out")).expect("the sandbox marker is made");
    let git = copy.path().join("README.in/.git");
    std::fs::create_dir_all(&git).expect("the project's repository is made");
    std::fs::write(git.join("HEAD"), "ref: refs/heads/main\n").expect("its branch is written");
    let cases = trycmd::TestCases::new();
    // `case` hands the cases back for chaining; they run when `cases` is dropped, at the end of the test.
    let _: &trycmd::TestCases = cases.case(&readme);
    drop(cases);
}
