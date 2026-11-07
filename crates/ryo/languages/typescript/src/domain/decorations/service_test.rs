//! A decorator's arguments are the text between the parentheses of its call, whatever comes between its callee and them.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::nodes::types::Node;
use crate::domain::syntax::{
    service::kind,
    types::{Dialect, Kind},
};
use crate::outbound::syntax::{factory, interface::SyntaxPort as _, types::SourceText};
use ryo::languages;
type Input = &'static str;
type Value = &'static str;
type Failure = ();
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_call_passes_what_is_between_its_parentheses"),
        word: Word::Succeeds,
        input: || "@Route(\"/x\", { method: \"get\" }) class C {}",
        expect: Expect::Value("\"/x\", { method: \"get\" }"),
    },
    Case {
        name: CaseName("a_decorator_that_is_no_call_passes_nothing"),
        word: Word::Succeeds,
        input: || "@sealed class C {}",
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_generic_call_passes_what_is_after_its_type_arguments"),
        word: Word::Succeeds,
        input: || "@dec<(a: number) => void>(1) class C {}",
        expect: Expect::Value("1"),
    },
    Case {
        name: CaseName("a_generic_call_without_arguments_passes_nothing"),
        word: Word::Succeeds,
        input: || "@dec<T>() class C {}",
        expect: Expect::Value(""),
    },
    Case {
        name: CaseName("a_call_of_a_member_passes_its_arguments"),
        word: Word::Succeeds,
        input: || "@a.b(2) class C {}",
        expect: Expect::Value("2"),
    },
    Case {
        name: CaseName("a_parenthesis_in_a_block_comment_before_the_call_is_skipped"),
        word: Word::Succeeds,
        input: || "@dec /* ( */ (\"arg\", 2) class C {}",
        expect: Expect::Value("\"arg\", 2"),
    },
    Case {
        name: CaseName("a_parenthesis_in_a_line_comment_before_the_call_is_skipped"),
        word: Word::Succeeds,
        input: || "@dec // (\n(3) class C {}",
        expect: Expect::Value("3"),
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
        .find(|at| kind(&tree, *at) == Kind::Node(Node::Decorator))
        .expect("case holds a decorator");
    let arguments = super::service::arguments(&tree, &mut symbols, at);
    languages::text_of(&symbols, bytes, arguments).map_or("", |text| {
        let value: &str = Box::leak(text.0.to_owned().into_boxed_str());
        value
    })
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
