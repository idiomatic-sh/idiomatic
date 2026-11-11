//! The support of the group's tests.

use super::super::super::service::classify;
use super::super::super::types::{LiteralClassification, Properties};
use crate::domain::{
    nodes::types::Node,
    semantic_syntax::service::positions,
    syntax::{service::kind, types::Kind},
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Tree};

pub(super) fn properties(source: &'static str) -> Vec<Properties> {
    let file = parsed(source.as_bytes()).expect("literal examples parse");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    positions(&tree)
        .filter(|at| kind(&tree, *at) == Kind::Node(Node::Constant))
        .filter_map(|at| match classify(&tree, at) {
            LiteralClassification::Known(properties) => Some(properties),
            LiteralClassification::NotPrimitive => None,
            LiteralClassification::MissingPayload => {
                panic!("a primitive literal has complete payload")
            }
        })
        .collect()
}

pub(super) fn primitive_values_use_parser_payload_and_decoded_tokens() -> String {
    let mut observed: Vec<String> = Vec::new();
    observed.push(format!("{:?}", properties(
            "0\n0x0\n0o0\n0b0\n1\n18446744073709551616\n0.0\n-0.0\n1e-9999\n5e-324\n1e9999\n0j\n1j\nTrue\nFalse\nNone\n...\nb''\nb'a'\nb'' b'\\x00'\n''\n'π'\n'' 'x'\n"
        )));
    observed.join("|")
}

pub(super) fn boolean_parser_payload_is_written_in_optimized_builds() -> String {
    let mut observed: Vec<String> = Vec::new();
    observed.push(format!("{:?}", properties("True\n")));
    observed.join("|")
}

pub(super) fn f_string_fragments_are_excluded_but_interpolation_expressions_are_literals() -> String
{
    let mut observed: Vec<String> = Vec::new();
    observed.push(format!("{:?}", properties("f'{0}x{\"\"!r:{1}}'\n")));
    observed.join("|")
}
