//! The `widths` service's cases: the width it counts for every kind of operation is the width the schema algebra answers.
use super::service::width_after;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::operation_schema::fixtures::{bindings, operations, tables};
use crate::domain::operation_schema::service::schema_of;
use crate::domain::operation_schema::types::InputShape;
use ryo_relations::domain::terms::types::Symbol;
use ryo_values::domain::plan_parts::types::RowWidth;
use ryo_values::domain::plans::service::kind_of;
use ryo_values::domain::query_operators::types::OperationKind;

type Input = ();
/// How many fixture operations the width was counted for and agreed with the schema, and the kinds it could not be counted for.
type Value = (usize, std::borrow::Cow<'static, [OperationKind]>);
/// The first kind whose counted width is not its schema's, with what the schema refused it for when it did.
type Failure = (
    OperationKind,
    Option<crate::domain::operating::errors::OperatingError>,
);

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("every_kind_but_the_hosts_rows_is_as_wide_as_its_schema"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((30, std::borrow::Cow::Borrowed(&[OperationKind::Hosted]))),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let mut agreed = 0;
    let mut uncounted = Vec::new();
    for operation in operations() {
        let inputs = tables(&operation, Symbol(0), true);
        let shapes: Vec<InputShape<'_>> = inputs
            .iter()
            .map(|table| InputShape {
                kinds: &table.kinds,
                tree: None,
            })
            .collect();
        let mut widths = [None; 3];
        for (width, table) in widths.iter_mut().zip(&inputs) {
            *width = Some(RowWidth(table.kinds.len()));
        }
        // The host's rows are an input of the schema algebra and no input of the plan: the plan does not say their width.
        if kind_of(&operation) == OperationKind::Hosted {
            widths = [None; 3];
        }
        let kind = kind_of(&operation);
        let schema = schema_of(&operation, &shapes, &bindings())
            .map_err(|error| (kind, Some(error)))?
            .len();
        match width_after(&operation, widths) {
            Some(counted) if counted == RowWidth(schema) => agreed += 1,
            Some(_) => return Err((kind, None)),
            None => uncounted.push(kind),
        }
    }
    Ok((agreed, uncounted.into()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
