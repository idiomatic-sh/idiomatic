//! The product README's Usage, each command run and its output compared.

#[test]
fn readme() {
    // The README sits beside the crate, and its commands run from the product root, `crates/ryo`.
    let readme = std::fs::canonicalize("../README.md").expect("the README sits beside the crate");
    std::env::set_current_dir(readme.parent().expect("the README has a directory"))
        .expect("the product root exists");
    let _: &trycmd::TestCases = trycmd::TestCases::new().case(readme);
}
