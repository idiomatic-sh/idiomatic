//! Public description and example of this operator.

use crate::domain::describing::constants::{described, field};
use ryo_values::domain::descriptions::constants::SELECT;
use ryo_values::domain::descriptions::types::{FieldRequirement, OperatorDescription};

/// Describe this operator's inputs, output and independent example.
pub(crate) fn description() -> OperatorDescription {
    described(
        SELECT,
        "Keep rows for which every comparison is true.",
        vec![
            field("from", "Input to filter.", FieldRequirement::Required),
            field(
                "where",
                "Typed comparisons of columns, literals, parameters or functions of them.",
                FieldRequirement::Required,
            ),
        ],
        &[
            "comparison family: equality",
            "comparison family: ordering",
            "comparison family: float-text",
            "comparison family: numeric, numeric_gte(value, 86400) compares canonical integer/binary64 text or integer cells without rounding integers; NaN and malformed text are false",
            "integer_successor(left, right): true only when right is exactly left plus one; canonical integer text or integer cells, no float coercion",
            "comparison family: membership",
            "comparison family: glob",
            "comparison family: affix, starts_with(path, \"src/\") or ends_with(name, \"Service\"), each side text",
            "function: concat(a, b, ...) joins two or more texts, concat(pascal(p.pkg), \"Service\") is TodoListsService for todo_lists",
            "function: pascal(text) writes todo_lists as TodoLists",
            "function: camel(text) writes todo_lists as todoLists",
            "function: snake(text) writes TodoLists as todo_lists",
            "function: kebab(text) writes TodoLists as todo-lists",
        ],
        r#"let example = select * from "function" where name = "example"; emit example subject file key(span);"#,

    )
}
