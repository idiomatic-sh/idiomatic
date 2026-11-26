//! Declared target selection is fixture setup, never a production source directive.
use super::select;

#[test]
fn malformed_and_unsupported_declarations_fail_without_a_default() {
    for text in [
        "# parse_options: not-json\nx = 1\n",
        "# parse_options: []\nx = 1\n",
        "# parse_options: {\"target-version\":\"3.13\",\"target-version\":\"3.14\"}\n",
        "# parse_options: {\"target-version\": null}\n",
        "# parse_options: {\"target-version\": 313}\n",
        "# parse_options: {\"target-version\": \"9.9\"}\n",
        "# parse_options {\"target-version\": \"3.13\"}\n",
        "# parse_options: {}\n# parse_options: {}\n",
    ] {
        assert!(select(text).is_err(), "{text}");
    }
    for text in ["x = 1\n", "# parse_options: {}\nx = 1\n"] {
        assert!(select(text).expect("no declared target is valid").is_none());
    }
}

#[test]
fn every_pinned_target_has_an_explicit_configuration() {
    for minor in 7..=15 {
        let source = format!("# parse_options: {{\"target-version\": \"3.{minor}\"}}\n");
        let selected = select(&source).expect("valid declaration").expect("explicit target");
        assert_eq!(selected.language.configuration_identity().0, format!("python-target=3.{minor}"));
    }
}
