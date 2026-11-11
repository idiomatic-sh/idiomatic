//! Parser-owned literal values classify without inferring runtime truthiness.
use super::fixtures::{properties, property};

#[test]
fn primitive_values_use_parser_payload_and_decoded_tokens() {
    assert_eq!(
        properties(
            "0\n0x0\n0o0\n0b0\n1\n18446744073709551616\n0.0\n-0.0\n1e-9999\n5e-324\n1e9999\n0j\n1j\nTrue\nFalse\nNone\n...\nb''\nb'a'\nb'' b'\\x00'\n''\n'π'\n'' 'x'\n"
        ),
        [
            property("integer", true, false, false),
            property("integer", true, false, false),
            property("integer", true, false, false),
            property("integer", true, false, false),
            property("integer", false, false, false),
            property("integer", false, false, false),
            property("float", true, false, false),
            property("float", true, false, false),
            property("float", true, false, false),
            property("float", false, false, false),
            property("float", false, false, false),
            property("complex", true, false, false),
            property("complex", false, false, false),
            property("boolean", false, false, true),
            property("boolean", false, false, false),
            property("null", false, false, false),
            property("ellipsis", false, false, false),
            property("bytes", false, true, false),
            property("bytes", false, false, false),
            property("bytes", false, false, false),
            property("string", false, true, false),
            property("string", false, false, false),
            property("string", false, false, false),
        ]
    );
}

#[test]
fn boolean_parser_payload_is_written_in_optimized_builds() {
    assert_eq!(
        properties("True\n"),
        [property("boolean", false, false, true)]
    );
}

#[test]
fn f_string_fragments_are_excluded_but_interpolation_expressions_are_literals() {
    assert_eq!(
        properties("f'{0}x{\"\"!r:{1}}'\n"),
        [
            property("integer", true, false, false),
            property("string", false, true, false),
            property("integer", false, false, false),
        ]
    );
}
