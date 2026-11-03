//! Legitimate path substitution and independent byte-preservation negatives.
use super::{
    service::expand,
    types::{OutputPart as Part, PathEncoding as Encoding},
};
use std::path::{Path, PathBuf};
#[cfg(unix)]
#[test]
fn exact_unicode_json_path_content_does_not_escape_unrelated_bytes() {
    let parts = [
        Part::Literal("{\"file\":\"".into()),
        Part::InputPath {
            file: "é\".py".into(),
            encoding: Encoding::JsonStringContent,
        },
        Part::Literal("\",\"assertion\":\"x < y\"}".into()),
    ];
    let actual = expand(&parts, Path::new("/tmp/日本"), &[PathBuf::from("é\".py")]).unwrap();
    assert_eq!(
        String::from_utf8(actual).unwrap(),
        "{\"file\":\"/tmp/日本/é\\\".py\",\"assertion\":\"x < y\"}"
    );
}
#[test]
fn unlisted_escaping_and_relative_root_slots_are_refused() {
    for file in ["other.py", "../case.py", "/case.py", ""] {
        let parts = [Part::InputPath {
            file: file.into(),
            encoding: Encoding::Utf8Bytes,
        }];
        assert!(expand(&parts, Path::new("/tmp/root"), &[PathBuf::from("case.py")]).is_err());
    }
    assert!(expand(&[], Path::new("relative"), &[]).is_err());
    assert!(expand(&[], Path::new(""), &[]).is_err());
}

#[test]
fn case_identity_framing_preserves_boundaries_and_roles() {
    use super::service::observation_key;
    assert_ne!(
        observation_key("a", "bc", "reference"),
        observation_key("ab", "c", "reference")
    );
    assert_ne!(
        observation_key("a", "bc", "reference"),
        observation_key("a", "bc", "candidate")
    );
    assert_eq!(
        observation_key("a", "../name", "reference").unwrap().len(),
        64
    );
}
