//! Exact parser projection cases and the shared syntax-port contract.
use super::adapter::SyntaxOxcParserAdapter;
use super::fixtures::walk::{reference, walked};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::syntax::service::{kind_name, kind_of};
use crate::domain::syntax::types::Goal;
use crate::outbound::syntax::fixtures::contract::check;
use crate::outbound::syntax::{interface::SyntaxPort as _, types::SourceText};
use ryo::languages::{
    self,
    contract::{Leafness, Walked},
};

#[derive(Clone, Copy, Debug)]
enum Given {
    Pinned,
    DifferentTree,
}
type Input = Given;
type Value = Vec<String>;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_pinned_vocabulary_example_matches_the_complete_tree"),
        word: Word::Succeeds,
        input: || Given::Pinned,
        expect: Expect::Value(Vec::new()),
    },
    Case {
        name: CaseName("a_changed_standard_tree_is_a_difference"),
        word: Word::Succeeds,
        input: || Given::DifferentTree,
        expect: Expect::Value(Vec::new()),
    },
];
fn flatten(tree: &Walked, depth: usize, output: &mut Vec<String>) {
    output.push(format!(
        "{depth} {:?} {} {} {}",
        tree.leaf,
        kind_name(kind_of(tree.kind)).0,
        tree.start.0,
        tree.end.0
    ));
    for child in &tree.children {
        flatten(child, depth + 1, output);
    }
}
fn compare(source: &str, replacement: Option<Walked>) -> Value {
    check(&SyntaxOxcParserAdapter);
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let mut builder = languages::table_builder(
        &mut elements,
        &mut symbols,
        languages::ByteSlice(source.as_bytes()),
    );
    SyntaxOxcParserAdapter.read(SourceText(source.as_bytes()), Goal::Detected, &mut builder);
    let Ok(expected) = replacement.map_or_else(|| reference(source), Ok) else {
        return if languages::diagnostics(&elements).next().is_some() {
            Vec::new()
        } else {
            vec![String::from(
                "reference refuses source, runtime emitted no diagnostic",
            )]
        };
    };
    let mut wanted = Vec::new();
    flatten(&expected, 0, &mut wanted);
    let actual: Vec<_> = languages::positions(&elements)
        .map(|at| {
            let (mut current, mut depth) = (at, 0_i32);
            while languages::parent_at(&elements, current) != current {
                current = languages::parent_at(&elements, current);
                depth += 1_i32;
            }
            let leaf = if languages::flags_at(&elements, at).0 & languages::TOKEN.0 == 0 {
                Leafness::Node
            } else {
                Leafness::Token
            };
            format!(
                "{depth} {leaf:?} {} {} {}",
                kind_name(kind_of(languages::kind_at(&elements, at))).0,
                languages::start_at(&elements, at).0,
                languages::end_at(&elements, at).0
            )
        })
        .collect();
    if actual == wanted {
        Vec::new()
    } else {
        vec![format!("expected {wanted:?}; actual {actual:?}")]
    }
}
fn subject(input: Input) -> Value {
    match input {
        Given::Pinned => {
            let standard =
                std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/standard");
            let text = std::fs::read_to_string(standard.join("samples.json"))
                .expect("pinned sources are readable");
            let samples: Vec<String> =
                serde_json::from_str(&text).expect("pinned samples are valid JSON");
            samples
                .iter()
                .flat_map(|source| {
                    compare(source, None)
                        .into_iter()
                        .map(move |difference| format!("{source:?}: {difference}"))
                })
                .collect()
        }
        Given::DifferentTree => {
            let mut changed = walked("a");
            changed.children.clear();
            if compare("a", Some(changed)).is_empty() {
                vec![String::from(
                    "the comparator accepted a tree missing its complete subtree",
                )]
            } else {
                Vec::new()
            }
        }
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
