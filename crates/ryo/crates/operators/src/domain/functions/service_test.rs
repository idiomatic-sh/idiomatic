//! The `functions` service's cases, on values alone: each function's text from the texts of its arguments.

use super::service::apply;
use super::types::MadeText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::Function;

/// A function and the texts of its arguments.
type Input = (Function, &'static [&'static str]);
/// The text the function makes over what the buffer held before.
type Value = &'static str;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("concat_writes_its_arguments_one_after_the_other"),
        word: Word::Succeeds,
        input: || (Function::Concat, &["Todos", "Service"]),
        expect: Expect::Value("TodosService"),
    },
    Case {
        name: CaseName("concat_writes_an_empty_argument_as_nothing"),
        word: Word::Succeeds,
        input: || (Function::Concat, &["Todos", "", "Service"]),
        expect: Expect::Value("TodosService"),
    },
    Case {
        name: CaseName("pascal_capitalises_one_word"),
        word: Word::Succeeds,
        input: || (Function::Pascal, &["todos"]),
        expect: Expect::Value("Todos"),
    },
    Case {
        name: CaseName("pascal_joins_the_words_of_snake_case"),
        word: Word::Succeeds,
        input: || (Function::Pascal, &["todo_lists"]),
        expect: Expect::Value("TodoLists"),
    },
    Case {
        name: CaseName("pascal_keeps_pascal_case"),
        word: Word::Succeeds,
        input: || (Function::Pascal, &["TodoLists"]),
        expect: Expect::Value("TodoLists"),
    },
    Case {
        name: CaseName("camel_lowers_the_first_word"),
        word: Word::Succeeds,
        input: || (Function::Camel, &["todo_lists"]),
        expect: Expect::Value("todoLists"),
    },
    Case {
        name: CaseName("snake_splits_pascal_case"),
        word: Word::Succeeds,
        input: || (Function::Snake, &["TodoLists"]),
        expect: Expect::Value("todo_lists"),
    },
    Case {
        name: CaseName("snake_splits_kebab_case"),
        word: Word::Succeeds,
        input: || (Function::Snake, &["todo-lists"]),
        expect: Expect::Value("todo_lists"),
    },
    Case {
        name: CaseName("kebab_splits_pascal_case"),
        word: Word::Succeeds,
        input: || (Function::Kebab, &["TodoLists"]),
        expect: Expect::Value("todo-lists"),
    },
    Case {
        name: CaseName("lower_preserves_separators_and_folds_unicode_case"),
        word: Word::Succeeds,
        input: || (Function::Lower, &["Ä.B_C"]),
        expect: Expect::Value("ä.b_c"),
    },
    Case {
        name: CaseName("a_case_of_empty_text_is_empty"),
        word: Word::Succeeds,
        input: || (Function::Pascal, &[""]),
        expect: Expect::Value(""),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let (function, texts) = input;
    let arguments: Vec<MadeText> = texts
        .iter()
        .map(|each| MadeText((*each).to_owned()))
        .collect();
    let mut made = MadeText(String::from("left over"));
    apply(function, &arguments, &mut made);
    Ok(Box::leak(made.0.into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
