//! Static names require every member of the selection to be static.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::TypeScriptNode;
use crate::domain::syntax::{
    service::kind,
    types::{Dialect, Kind},
};
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;
type Input = &'static str;
type Value = Option<&'static str>;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_dotted_call_has_a_static_target"),
        word: Word::Succeeds,
        input: || "object.method()",
        expect: Expect::Value(Some("object.method")),
    },
    Case {
        name: CaseName("computed_members_have_no_static_target"),
        word: Word::Succeeds,
        input: || "object[index]()",
        expect: Expect::Value(None),
    },
    Case {
        name: CaseName("dynamic_prefix_does_not_become_a_static_suffix"),
        word: Word::Succeeds,
        input: || "object[index].method()",
        expect: Expect::Value(None),
    },
];
fn subject(source: Input) -> Value {
    let (mut elements, mut symbols) = (
        languages::Elements::default(),
        languages::Symbols::default(),
    );
    let bytes = languages::ByteSlice(source.as_bytes());
    factory::oxc_parser().read(
        SourceText(source.as_bytes()),
        Dialect::TypeScript,
        &mut languages::table_builder(&mut elements, &mut symbols, bytes),
    );
    let tree = languages::Tree {
        elements: &elements,
        source: bytes,
        file: languages::FileId(0),
    };
    let at = languages::positions(&elements)
        .find(|at| kind(&tree, *at) == Kind::Node(TypeScriptNode::CallExpression))
        .expect("case contains a call");
    let name = super::service::target(&tree, &mut symbols, at);
    languages::text_of(&symbols, bytes, name).map(|name| {
        let value: &str = Box::leak(name.0.to_owned().into_boxed_str());
        value
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
