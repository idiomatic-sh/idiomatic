//! Declared target selection is fixture setup, never a production source directive.
use super::select;
use ryo::languages::contract::{OracleRows, Shape, build};
use ryo::languages::{
    self, ByteSlice, Elements, FieldRequest, FileId, NoteRequest, ParseRequest, SourcePath, Symbols,
};

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

/// Build through the selected real SDK parser, without executing the supplied Python code.
fn syntax_errors(context: &languages::contract::ConfiguredFixture, source: &str) -> usize {
    let mut elements = Elements::default();
    build(
        context.language.parse_engine().as_ref(),
        ParseRequest {
            path: SourcePath("case.py"),
            file: FileId(0),
            bytes: ByteSlice(source.as_bytes()),
            notes: NoteRequest::Unwanted,
            fields: FieldRequest::Wanted,
        },
        &mut elements,
        &mut Symbols::default(),
    )
    .expect("fixture is recognized");
    languages::diagnostic_count(&elements).0
}

#[test]
fn actual_parser_and_reference_callbacks_share_the_declared_target() {
    for (target, rejected) in [("3.13", false), ("3.14", true)] {
        let source =
            format!("# parse_options: {{\"target-version\": \"{target}\"}}\nx: (y := int)\n");
        let selected = select(&source)
            .expect("valid declaration")
            .expect("explicit target");
        assert_eq!(
            selected.language.configuration_identity().0,
            format!("python-target={target}")
        );
        assert_eq!(syntax_errors(&selected, &source) > 0, rejected);
        let Shape::Walk(reference) = selected.shape else {
            panic!("Python selects its fixture walk");
        };
        let walked = reference(&source);
        assert!(
            !walked.children.is_empty(),
            "configured shape must actually run"
        );
        assert!((selected.oracle)(&source, &OracleRows(Vec::new())).is_empty());
    }
}

#[test]
fn every_pinned_target_has_an_explicit_configuration() {
    for minor in 7..=15 {
        let source = format!("# parse_options: {{\"target-version\": \"3.{minor}\"}}\n");
        let selected = select(&source)
            .expect("supported version")
            .expect("declared version");
        assert_eq!(
            selected.language.configuration_identity().0,
            format!("python-target=3.{minor}")
        );
    }
}
