//! The `functions` service: the text a function makes from the texts of its arguments, with no IO and no ports.

use heck::{ToKebabCase as _, ToLowerCamelCase as _, ToSnakeCase as _, ToUpperCamelCase as _};

use super::types::MadeText;
use ryo_values::domain::plan_parts::types::Function;

/// Writes into `made` the text `function` makes of `arguments`: `concat` writes them one after the other, and a case writes its
/// one argument in that case, `todo_lists` as `TodoLists`, `todoLists`, `todo_lists` or `todo-lists`.
pub(crate) fn apply(function: Function, arguments: &[MadeText], made: &mut MadeText) {
    made.0.clear();
    if function == Function::Concat {
        for each in arguments {
            made.0.push_str(&each.0);
        }
        return;
    }
    let Some(first) = arguments.first() else {
        return;
    };
    let cased = match function {
        Function::Pascal => first.0.to_upper_camel_case(),
        Function::Camel => first.0.to_lower_camel_case(),
        Function::Snake => first.0.to_snake_case(),
        Function::Kebab => first.0.to_kebab_case(),
        Function::Lower => first.0.to_lowercase(),
        Function::Concat => return,
    };
    made.0.push_str(&cased);
}
