//! The scenarios of the group: each runs its subject and records what it observed.
use super::super::super::service::{key_of, prepare};
use crate::domain::{
    model::types::Declarations,
    semantic_syntax::{
        service::{positions, visibility},
        types::Visibility,
    },
    syntax::service::{kind, written},
    syntax::types::Kind,
};
use crate::fixtures::reading::{initialize_model, parsed};
use ryo::languages::{self, ByteSlice, Tree};

pub(super) fn supported_numeric_keys_normalize_and_unsupported_expressions_are_absent() -> String {
    let mut observed: Vec<String> = Vec::new();
    let file = parsed(b"1\nTrue\n1.0\n0x1\n1+0j\n1+1\n(f(), 2)\n(1, (2,))\n(1.0, (2,))\n(18446744073709551616, 1)\n(18446744073709551616, 1.0)\n(-18446744073709551616, 1)\n(-18446744073709551616, 1.0)\n18446744073709551615\n0xffffffffffffffff\n1_000\n0x3e8\n-True\n-1\n")
        .expect("independent source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(&tree, &model.constant_keys, &mut symbols, |at, symbols| {
        (visibility(&tree, &model, at, symbols) == Visibility::Visible).into()
    });
    observed.push(format!("{:?}", index.state));
    let occurrences: Vec<_> = positions(&tree)
        .filter(|at| {
            matches!(
                kind(&tree, *at),
                Kind::Node(
                    crate::domain::nodes::types::Node::Constant
                        | crate::domain::nodes::types::Node::Tuple
                        | crate::domain::nodes::types::Node::UnaryOp
                        | crate::domain::nodes::types::Node::BinOp
                )
            )
        })
        .map(|at| (written(&tree, at).0, key_of(&index, at)))
        .collect();
    let find = |source: &str| {
        occurrences
            .iter()
            .find(|(text, _)| *text == source)
            .and_then(|(_, key)| *key)
    };
    let equivalent = |left: &str, right: &str| match (find(left), find(right)) {
        (Some(left), Some(right)) => format!("{}", left == right),
        missing => format!("{missing:?}"),
    };
    observed.push(equivalent("1", "True"));
    observed.push(equivalent("1", "1.0"));
    observed.push(equivalent("1", "0x1"));
    observed.push(equivalent("1", "1+0j"));
    observed.push(equivalent("18446744073709551615", "0xffffffffffffffff"));
    observed.push(equivalent("1_000", "0x3e8"));
    observed.push(equivalent("-True", "-1"));
    observed.push(equivalent(
        "(18446744073709551616, 1)",
        "(18446744073709551616, 1.0)",
    ));
    observed.push(format!("{}", find("1+1").is_none()));
    observed.push(format!("{}", find("(f(), 2)").is_none()));
    observed.push(format!("{}", find("(-18446744073709551616, 1)").is_none()));
    observed.push(format!(
        "{}",
        find("(-18446744073709551616, 1.0)").is_none()
    ));
    observed.push(equivalent("(1, (2,))", "(1.0, (2,))"));
    observed.join("|")
}

pub(super) fn float_boundaries_and_huge_integer_structural_keys_are_retained() -> String {
    let mut observed: Vec<String> = Vec::new();
    let file = parsed(b"9007199254740992\n9007199254740992.0\n9007199254740993\n9007199254740993.0\n18446744073709551616\n18446744073709551616\n-0.0\n0.0\n")
        .expect("independent source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(&tree, &model.constant_keys, &mut symbols, |at, symbols| {
        (visibility(&tree, &model, at, symbols) == Visibility::Visible).into()
    });
    let values: Vec<_> = positions(&tree)
        .filter(|at| {
            matches!(
                kind(&tree, *at),
                Kind::Node(
                    crate::domain::nodes::types::Node::Constant
                        | crate::domain::nodes::types::Node::UnaryOp
                )
            )
        })
        .map(|at| (written(&tree, at).0, key_of(&index, at)))
        .collect();
    let wanted = [
        "9007199254740992",
        "9007199254740992.0",
        "9007199254740993",
        "9007199254740993.0",
        "18446744073709551616",
        "18446744073709551616",
        "-0.0",
        "0.0",
    ];
    let keys: Vec<_> = wanted
        .iter()
        .map(|wanted| {
            values
                .iter()
                .find(|(text, _)| text == wanted)
                .and_then(|(_, key)| *key)
        })
        .collect();
    let missing: Vec<_> = wanted
        .iter()
        .zip(&keys)
        .filter(|(_, key)| key.is_none())
        .map(|(spelling, _)| *spelling)
        .collect();
    observed.push(format!("{missing:?}"));
    observed.push(format!("{:?}", keys[0]));
    observed.push(format!("{}", keys[2] != keys[3]));
    let huge: Vec<_> = values
        .iter()
        .filter(|(text, _)| *text == "18446744073709551616")
        .filter_map(|(_, key)| *key)
        .collect();
    observed.push(format!("{:?}", huge.len()));
    observed.push(format!("{:?}", huge.first()));
    observed.push(format!("{:?}", keys[6]));
    observed.join("|")
}

pub(super) fn underflow_subnormal_infinity_complex_and_unary_values_follow_typed_payloads() -> String
{
    let mut observed: Vec<String> = Vec::new();
    let source = b"0\n1e-9999\n5e-324\n1e9999\n0j\n1j\n1+1j\n1.0+1j\n1-1j\n1.0-1j\n9007199254740993+0j\n9007199254740992\n-0.0\n0.0\n18446744073709551615\n18446744073709551616.0\n";
    let file = parsed(source).expect("independent source parses");
    let tree = Tree {
        elements: &file.elements,
        source: ByteSlice(file.bytes),
        file: languages::FileId(0),
    };
    let mut symbols = file.symbols;
    let mut model = Declarations::default();
    initialize_model(&tree, &mut model);
    let index = prepare(&tree, &model.constant_keys, &mut symbols, |at, symbols| {
        (visibility(&tree, &model, at, symbols) == Visibility::Visible).into()
    });
    let occurrences: Vec<_> = positions(&tree)
        .filter(|at| {
            matches!(
                kind(&tree, *at),
                Kind::Node(
                    crate::domain::nodes::types::Node::Constant
                        | crate::domain::nodes::types::Node::UnaryOp
                        | crate::domain::nodes::types::Node::BinOp
                )
            )
        })
        .map(|at| (written(&tree, at).0, key_of(&index, at)))
        .collect();
    let find = |spelling: &str| {
        occurrences
            .iter()
            .find(|(text, _)| *text == spelling)
            .and_then(|(_, key)| *key)
    };
    let key = |spelling: &str| format!("{:?}", find(spelling));
    let distinct = |left: &str, right: &str| match (find(left), find(right)) {
        (Some(left), Some(right)) => format!("{}", left != right),
        missing => format!("{missing:?}"),
    };
    observed.push(key("0"));
    observed.push(distinct("0", "5e-324"));
    observed.push(distinct("0", "1e9999"));
    observed.push(key("0"));
    observed.push(distinct("0", "1j"));
    observed.push(key("1+1j"));
    observed.push(key("1-1j"));
    observed.push(key("9007199254740993+0j"));
    observed.push(key("-0.0"));
    observed.push(distinct("18446744073709551615", "18446744073709551616.0"));
    observed.join("|")
}

pub(super) fn invalidation_clears_occurrences_but_keeps_the_reader_reusable() -> String {
    let mut observed: Vec<String> = Vec::new();
    let mut model = Declarations::default();
    let first = parsed(b"1\n").expect("source parses");
    let first_tree = Tree {
        elements: &first.elements,
        source: ByteSlice(first.bytes),
        file: languages::FileId(0),
    };
    let mut first_symbols = first.symbols;
    initialize_model(&first_tree, &mut model);
    observed.push(format!(
        "{}",
        model.constant_keys.borrow().records.is_empty()
    ));
    let first_index = prepare(
        &first_tree,
        &model.constant_keys,
        &mut first_symbols,
        |at, symbols| (visibility(&first_tree, &model, at, symbols) == Visibility::Visible).into(),
    );
    let first_words = first_index.words.clone();
    observed.push(format!("{}", !first_index.records.is_empty()));
    drop(first_index);

    languages::clear_keys(model.constant_keys.get_mut());
    let second = parsed(b"2\n").expect("replacement source parses");
    let second_tree = Tree {
        elements: &second.elements,
        source: ByteSlice(second.bytes),
        file: languages::FileId(0),
    };
    let mut second_symbols = second.symbols;
    initialize_model(&second_tree, &mut model);
    let second_index = prepare(
        &second_tree,
        &model.constant_keys,
        &mut second_symbols,
        |at, symbols| (visibility(&second_tree, &model, at, symbols) == Visibility::Visible).into(),
    );
    observed.push(format!("{}", !second_index.records.is_empty()));
    observed.push(format!("{}", second_index.words != first_words));
    observed.join("|")
}
