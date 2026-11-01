//! The `model` service's cases, on values alone.

use ryo::languages::{Position, Symbol};
use std::cell::RefCell;

use super::service::redirect_of;
use super::types::{BindingRedirect, Declarations, ScopeDeclaration};
use crate::cases::{Case, CaseName, Expect, Word, run};

/// The scope and the name asked about.
type Input = (Position, Symbol);
/// Where the scope binds the name instead of in itself, when it says.
type Value = Option<BindingRedirect>;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_global_name_is_bound_in_the_module"),
        word: Word::Succeeds,
        input: || (Position(3), Symbol(1)),
        expect: Expect::Value(Some(BindingRedirect::Global)),
    },
    Case {
        name: CaseName("a_nonlocal_name_is_bound_in_the_enclosing_function"),
        word: Word::Succeeds,
        input: || (Position(7), Symbol(1)),
        expect: Expect::Value(Some(BindingRedirect::Nonlocal)),
    },
    Case {
        name: CaseName("a_name_another_scope_declares_is_bound_where_it_is"),
        word: Word::Succeeds,
        input: || (Position(5), Symbol(1)),
        expect: Expect::Value(None),
    },
];

fn subject((scope, name): Input) -> Value {
    let model = Declarations {
        language_target: (3, 15),
        bindings: Vec::default(),
        expression_shapes: RefCell::default(),
        constant_keys: RefCell::default(),
        format_parsing: RefCell::default(),
        brace_format: RefCell::default(),
        indentation: RefCell::default(),
        qualification: RefCell::default(),
        semantic: RefCell::default(),
        binding_analysis: RefCell::default(),
        declared: vec![
            ScopeDeclaration {
                scope: Position(3),
                name: Symbol(1),
                redirect: BindingRedirect::Global,
            },
            ScopeDeclaration {
                scope: Position(7),
                name: Symbol(1),
                redirect: BindingRedirect::Nonlocal,
            },
        ],
    };
    redirect_of(&model, scope, name)
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
