//! The process spelling participates in existing invocation and case identities.
use super::value::ProjectContextMode;
#[test]
fn explicit_context_modes_preserve_wire_identity_and_default_isolation() {
    assert_eq!(ProjectContextMode::default(), ProjectContextMode::Isolated);
    assert_eq!(ProjectContextMode::Isolated.as_str(), "isolated");
    assert_eq!(ProjectContextMode::ListedDisk.as_str(), "listed-disk");
}
