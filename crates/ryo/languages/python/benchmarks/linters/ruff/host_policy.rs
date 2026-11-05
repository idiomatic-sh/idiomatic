//! Pure host applicability policy, shared by the executable and its test target.

#[cfg(unix)]
pub(crate) fn supported(microsoft: bool, container: bool) -> bool {
    !microsoft || container
}

#[cfg(all(test, unix))]
#[test]
fn wsl_reference_policy_does_not_misclassify_containers_or_rewrite_facts() {
    assert!(supported(false, false));
    assert!(!supported(true, false));
    assert!(supported(true, true));
}
