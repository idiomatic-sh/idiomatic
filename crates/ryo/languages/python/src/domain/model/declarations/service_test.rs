//! The `declarations` service's cases: where the innermost function of a parsed source binds a name.

use super::service::{redirect_of, redirected};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::lexical::service::span_of;
use crate::domain::model::lexical::types::{NodeClass, ScopeRedirect};
use crate::domain::model::names::service::find;
use crate::domain::model::view::types::{ModelPart, ModelView as _};
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::{FileId, NameText, Position};

/// The source and the name asked about in its last function.
type Input = (&'static str, &'static str);
/// Where that function binds the name instead of in itself, when it says.
type Value = Option<ScopeRedirect>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_global_name_is_bound_in_the_module"),
        word: Word::Succeeds,
        input: || ("def f():\n    global x\n    x = 1\n", "x"),
        expect: Expect::Value(Some(ScopeRedirect::Global)),
    },
    Case {
        name: CaseName("a_nonlocal_name_is_bound_in_the_enclosing_function"),
        word: Word::Succeeds,
        input: || ("def f():\n    x = 0\n    def g():\n        nonlocal x\n        x = 1\n", "x"),
        expect: Expect::Value(Some(ScopeRedirect::Nonlocal)),
    },
    Case {
        name: CaseName("the_first_declaration_of_a_name_holds"),
        word: Word::Succeeds,
        input: || ("def f():\n    def g():\n        nonlocal x\n        global x\n", "x"),
        expect: Expect::Value(Some(ScopeRedirect::Nonlocal)),
    },
    Case {
        name: CaseName("an_undeclared_name_is_bound_where_it_is"),
        word: Word::Succeeds,
        input: || ("def f():\n    global x\n    y = 1\n", "y"),
        expect: Expect::Value(None),
    },
];

fn subject((source, name): Input) -> Value {
    let mut answer = None;
    viewed(&mut ModelFixture::default(), source, FileId(0), &mut |view| {
        let model = view.semantics(ModelPart::Lexical).model;
        let function = model.lexical.class.iter().rposition(|class| *class == NodeClass::FunctionDef);
        let code = find(&model.names, NameText(name));
        if let (Some(function), Some(code)) = (function, code) {
            answer = redirect_of(&model.lexical, Position(u32::try_from(function).unwrap_or(u32::MAX)), code);
        }
    });
    answer
}

/// The first line of the scope the last function of the source binds the name in.
type Where = &'static str;

/// The rows of the namespace a `nonlocal` name is bound in.
const NAMESPACES: &[Case<Input, Where, Failure>] = &[
    Case {
        name: CaseName("a_nonlocal_name_is_bound_in_the_function_around_that_binds_it"),
        word: Word::Succeeds,
        input: || ("def f():\n    x = 0\n    def g():\n        nonlocal x\n        x = 1\n", "x"),
        expect: Expect::Value("def f():"),
    },
    Case {
        name: CaseName("a_nonlocal_name_skips_a_function_around_that_does_not_bind_it"),
        word: Word::Succeeds,
        input: || {
            (
                "def test():\n    name = 1\n    def decorator():\n        def inner():\n            nonlocal name\n            name = 2\n",
                "name",
            )
        },
        expect: Expect::Value("def test():"),
    },
    Case {
        name: CaseName("a_nonlocal_name_passes_through_a_function_that_declares_it_nonlocal_too"),
        word: Word::Succeeds,
        input: || {
            (
                "def f():\n    x = 0\n    def g():\n        nonlocal x\n        def h():\n            nonlocal x\n            x = 1\n",
                "x",
            )
        },
        expect: Expect::Value("def f():"),
    },
];

fn bound_in((source, name): Input) -> Where {
    let mut answer = "";
    viewed(&mut ModelFixture::default(), source, FileId(0), &mut |view| {
        let tree = view.tree;
        let model = view.semantics(ModelPart::Lexical).model;
        let function = model.lexical.class.iter().rposition(|class| *class == NodeClass::FunctionDef);
        let code = find(&model.names, NameText(name));
        if let (Some(function), Some(code)) = (function, code) {
            let scope = redirected(&model.lexical, Position(u32::try_from(function).unwrap_or(u32::MAX)), code);
            let span = span_of(tree, scope);
            let text = source
                .get(usize::try_from(span.start.0).unwrap_or(0)..usize::try_from(span.end.0).unwrap_or(0))
                .unwrap_or_default();
            answer = text.lines().next().unwrap_or_default();
        }
    });
    // A test-only rendering: the source is static, so its line is.
    Box::leak(answer.to_owned().into_boxed_str())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
    run(NAMESPACES, Word::Succeeds, |input| Ok(bound_in(input)));
}
