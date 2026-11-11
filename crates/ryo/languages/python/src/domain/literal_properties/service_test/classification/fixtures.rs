//! The support of the group's tests.
use super::super::super::service::classify;
use super::super::super::types::{LiteralClassification, Properties};
use crate::domain::{
    nodes::types::Node,
    semantic_syntax::service::positions,
    syntax::{service::kind, types::Kind},
};
use crate::fixtures::reading::parsed;
use ryo::languages::{self, ByteSlice, Flag, NameText, Tree};

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

pub(super) fn property(kind: &'static str, zero: bool, empty: bool, boolean: bool) -> Properties {
    let flag = |raised: bool| if raised { Flag::Raised } else { Flag::Lowered };
    Properties {
        kind: NameText(kind),
        zero: flag(zero),
        empty: flag(empty),
        boolean: flag(boolean),
    }
}
