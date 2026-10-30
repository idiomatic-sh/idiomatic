//! The `refinements` service's cases, on values alone: the word each declaration and each kind of parameter is carried as.

use super::service::{positional_name, receiver_name, refinement_name};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::refinements::types::Declared;

/// What a row of the cases asks of the refinements.
#[derive(Clone, Copy)]
enum Scenario {
    /// The word of every kind of declaration, in the order of `Declared`.
    Declarations,
    /// The word of a receiver, then of a parameter passed by position.
    Parameters,
}

/// What a row of the cases asks of the refinements.
type Input = Scenario;
/// The words, one for each thing asked.
type Value = &'static [&'static str];
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("every_declaration_is_carried_as_its_own_word"),
        word: Word::Succeeds,
        input: || Scenario::Declarations,
        expect: Expect::Value(&[
            "function",
            "struct",
            "enum",
            "trait",
            "variable",
            "const",
            "static",
            "module",
            "type-alias",
            "union",
            "macro-rules",
            "macro",
            "method",
            "associated-function",
            "variant",
            "field",
            "tuple-field",
            "type-parameter",
            "lifetime-parameter",
            "const-parameter",
            "use",
            "extern-crate",
        ]),
    },
    Case {
        name: CaseName("a_parameter_is_a_receiver_or_positional"),
        word: Word::Succeeds,
        input: || Scenario::Parameters,
        expect: Expect::Value(&["receiver", "positional"]),
    },
];

fn subject(scenario: Input) -> Value {
    let words: Vec<&'static str> = match scenario {
        Scenario::Declarations => [
            Declared::Function,
            Declared::Struct,
            Declared::Enum,
            Declared::Trait,
            Declared::Variable,
            Declared::Const,
            Declared::Static,
            Declared::Module,
            Declared::TypeAlias,
            Declared::Union,
            Declared::MacroRules,
            Declared::Macro,
            Declared::Method,
            Declared::AssociatedFunction,
            Declared::Variant,
            Declared::Field,
            Declared::TupleField,
            Declared::TypeParameter,
            Declared::LifetimeParameter,
            Declared::ConstParameter,
            Declared::Use,
            Declared::ExternCrate,
        ]
        .into_iter()
        .map(|declared| refinement_name(declared).0)
        .collect(),
        Scenario::Parameters => vec![receiver_name().0, positional_name().0],
    };
    Box::leak(words.into_boxed_slice())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
