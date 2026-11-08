//! The `functions` service's cases, on values alone: each function's text from the texts of its arguments.

use super::service::apply;
use super::types::MadeText;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::Function;

/// A function, the texts of its arguments and the text it is expected to make.
type Input = (Function, &'static [&'static str], &'static str);
/// 1 when the function makes the expected text over what the buffer held before; a row whose text differs fails.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("concat_writes_its_arguments_one_after_the_other"),
        word: Word::Succeeds,
        input: || (Function::Concat, &["Todos", "Service"], "TodosService"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("concat_writes_an_empty_argument_as_nothing"),
        word: Word::Succeeds,
        input: || (Function::Concat, &["Todos", "", "Service"], "TodosService"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("pascal_capitalises_one_word"),
        word: Word::Succeeds,
        input: || (Function::Pascal, &["todos"], "Todos"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("pascal_joins_the_words_of_snake_case"),
        word: Word::Succeeds,
        input: || (Function::Pascal, &["todo_lists"], "TodoLists"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("pascal_keeps_pascal_case"),
        word: Word::Succeeds,
        input: || (Function::Pascal, &["TodoLists"], "TodoLists"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("camel_lowers_the_first_word"),
        word: Word::Succeeds,
        input: || (Function::Camel, &["todo_lists"], "todoLists"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("snake_splits_pascal_case"),
        word: Word::Succeeds,
        input: || (Function::Snake, &["TodoLists"], "todo_lists"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("snake_splits_kebab_case"),
        word: Word::Succeeds,
        input: || (Function::Snake, &["todo-lists"], "todo_lists"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("kebab_splits_pascal_case"),
        word: Word::Succeeds,
        input: || (Function::Kebab, &["TodoLists"], "todo-lists"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("lower_preserves_separators_and_folds_unicode_case"),
        word: Word::Succeeds,
        input: || (Function::Lower, &["Ä.B_C"], "ä.b_c"),
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_case_of_empty_text_is_empty"),
        word: Word::Succeeds,
        input: || (Function::Pascal, &[""], ""),
        expect: Expect::Value(1),
    },
];

fn subject(input: Input) -> Result<Value, Failure> {
    let (function, texts, expected) = input;
    let arguments: Vec<MadeText> = texts
        .iter()
        .map(|each| MadeText((*each).to_owned()))
        .collect();
    let mut made = MadeText(String::from("left over"));
    apply(function, &arguments, &mut made);
    if made.0 == expected { Ok(1) } else { Err(()) }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
