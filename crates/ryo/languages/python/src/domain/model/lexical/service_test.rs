//! The `lexical` service's cases: what a class of node is to the name rules, on values, and the scope a node of a parsed
//! source is evaluated in.

use super::service::{scope_at, scope_form, syntax_form};
use super::types::{NodeClass, ScopeForm, SyntaxForm};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::model::view::types::{ModelPart, ModelView as _};
use crate::outbound::syntax::fixtures::model_view::{ModelFixture, viewed};
use ryo::languages::{FileId, Position};

/// What a row asks.
type Input = Question;
/// What a row expects.
type Value = Answer;

/// A class of node, or a source whose scopes are asked.
#[derive(Clone, Copy, Debug)]
enum Question {
    /// What a class of node is.
    Class(NodeClass),
    /// The class of the scope each name expression of the source is evaluated in, in source order.
    Scopes(&'static str),
}

/// What the class is, or the classes of the scopes.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Answer {
    /// How the class opens a namespace, and whether it is a statement or an expression.
    Class(Option<ScopeForm>, SyntaxForm),
    /// The classes of the scopes.
    Scopes(&'static [NodeClass]),
}
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_function_is_a_statement_that_opens_a_function_scope"),
        word: Word::Succeeds,
        input: || Question::Class(NodeClass::FunctionDef),
        expect: Expect::Value(Answer::Class(
            Some(ScopeForm::Function),
            SyntaxForm::Statement,
        )),
    },
    Case {
        name: CaseName("a_set_comprehension_is_an_expression_that_opens_a_comprehension_scope"),
        word: Word::Succeeds,
        input: || Question::Class(NodeClass::SetComp),
        expect: Expect::Value(Answer::Class(
            Some(ScopeForm::Comprehension),
            SyntaxForm::Expression,
        )),
    },
    Case {
        name: CaseName("a_parameter_opens_nothing_and_is_neither"),
        word: Word::Succeeds,
        input: || Question::Class(NodeClass::Parameter),
        expect: Expect::Value(Answer::Class(None, SyntaxForm::Neither)),
    },
    Case {
        name: CaseName(
            "a_default_a_decorator_and_a_first_iterable_are_evaluated_around_their_scope",
        ),
        word: Word::Succeeds,
        input: || {
            Question::Scopes(
                "@deco\ndef f(a=default):\n    return [inner for inner in outer if inner]\n",
            )
        },
        expect: Expect::Value(Answer::Scopes(&[
            NodeClass::Module,
            NodeClass::Module,
            NodeClass::ListComp,
            NodeClass::ListComp,
            NodeClass::FunctionDef,
            NodeClass::ListComp,
        ])),
    },
];

fn subject(input: Input) -> Value {
    match input {
        Question::Class(class) => Answer::Class(scope_form(class), syntax_form(class)),
        Question::Scopes(source) => {
            let mut scopes = Vec::new();
            viewed(
                &mut ModelFixture::default(),
                source,
                FileId(0),
                &mut |view| {
                    let lexical = &view.semantics(ModelPart::Lexical).model.lexical;
                    for (number, class) in lexical.class.iter().enumerate() {
                        if *class == NodeClass::Name {
                            let scope = scope_at(
                                lexical,
                                Position(u32::try_from(number).unwrap_or(u32::MAX)),
                            );
                            scopes.push(
                                lexical
                                    .class
                                    .get(usize::from(scope))
                                    .copied()
                                    .unwrap_or_default(),
                            );
                        }
                    }
                },
            );
            Answer::Scopes(Box::leak(scopes.into_boxed_slice()))
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
