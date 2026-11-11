//! Invalid bound and row patterns stay lazy until actual rows reach the public owner.
use super::super::super::fixtures;
use crate::inbound::use_cases::execution::interface::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, LiteralText, Operand, ParameterId, PlanComparison, Test},
    query_bindings::types::ParameterBinding,
    rows::types::{RowCount, Table},
};
use std::borrow::Cow;

/// Where the invalid regex `[` comes from.
#[derive(Clone, Copy, Debug)]
enum Pattern {
    /// A bound parameter.
    Bound,
    /// The row's second column.
    Row,
}

/// What the select of `matches(column 0, pattern)` shows: the schema it is given, the rows it
/// answers over an empty table, and the regex compiler's reason when a row reaches it.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Lazy {
    schema: Cow<'static, [CellKind]>,
    empty_rows: usize,
    invalid_regex: Option<Cow<'static, str>>,
}

/// The regex compiler's reason for `[`.
const UNCLOSED_CLASS: &str = "regex parse error:\n    [\n    ^\nerror: unclosed character class";

type Input = Pattern;
type Value = Lazy;
type Failure = OperatingError;

/// Selects function rows whose first column matches the pattern, first over no rows, then over one.
fn execution_boundary(pattern: Pattern) -> Result<Value, Failure> {
    use ryo_execution_domain::domain::operation_schema::{service::schema_of, types::InputShape};
    use ryo_language::domain::symbols::{
        service::intern_derived,
        types::{ByteSlice, Symbols},
    };
    use ryo_relations::domain::relations::types::Value as Cell;
    use ryo_relations::domain::terms::types::NameText;
    use ryo_values::domain::plans::types::Operation;
    let mut symbols = Symbols::default();
    let word = intern_derived(&mut symbols, ByteSlice(b""), NameText("word"));
    let bad = intern_derived(&mut symbols, ByteSlice(b""), NameText("["));
    let dynamic = matches!(pattern, Pattern::Row);
    let kinds: &'static [CellKind] = if dynamic {
        &[CellKind::Symbol, CellKind::Symbol]
    } else {
        &[CellKind::Symbol]
    };
    let comparison = PlanComparison {
        test: Test::Matches,
        left: Operand::Column(ColumnIndex(0)),
        right: if dynamic {
            Operand::Column(ColumnIndex(1))
        } else {
            Operand::Parameter(ParameterId(0))
        },
        origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
    };
    let operation = Operation::Select {
        from: ryo_values::domain::plan_parts::types::Input::Relation(Relation::Function),
        comparisons: vec![comparison],
        kept: Vec::new(),
    };
    let bindings = [ParameterBinding::Text(LiteralText("[".into()))];
    let metadata = [InputShape {
        kinds: &kinds,
        tree: None,
    }];
    let schema = schema_of(&operation, &metadata, &bindings)?;
    let empty = Table {
        kinds: kinds.to_vec(),
        ..Table::default()
    };
    let answer = fixtures::requests::table_request(
        operation.clone(),
        &[&empty],
        bindings.to_vec(),
        &mut symbols,
    )?;

    let populated = Table {
        kinds: kinds.to_vec(),
        count: RowCount(1),
        cells: if dynamic {
            vec![Cell::Symbol(word), Cell::Symbol(bad)]
        } else {
            vec![Cell::Symbol(word)]
        },
    };
    let reached = fixtures::requests::table_request(
        operation,
        &[&populated],
        bindings.to_vec(),
        &mut symbols,
    );
    let invalid_regex = match reached {
        Err(OperatingError::InvalidRegex { reason }) => Some(Cow::Owned(reason)),
        Err(other) => return Err(other),
        Ok(_) => None,
    };
    Ok(Lazy {
        schema: Cow::Owned(schema.to_vec()),
        empty_rows: answer.count.0,
        invalid_regex,
    })
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("bound_regex_fails_only_when_its_row_is_executed"),
        word: Word::Succeeds,
        input: || Pattern::Bound,
        expect: Expect::Value(Lazy {
            schema: Cow::Borrowed(&[CellKind::Symbol]),
            empty_rows: 0,
            invalid_regex: Some(Cow::Borrowed(UNCLOSED_CLASS)),
        }),
    },
    Case {
        name: CaseName("row_regex_fails_only_when_its_row_is_executed"),
        word: Word::Succeeds,
        input: || Pattern::Row,
        expect: Expect::Value(Lazy {
            schema: Cow::Borrowed(&[CellKind::Symbol, CellKind::Symbol]),
            empty_rows: 0,
            invalid_regex: Some(Cow::Borrowed(UNCLOSED_CLASS)),
        }),
    },
];
fn subject(pattern: Input) -> Result<Value, Failure> {
    execution_boundary(pattern)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

use ryo_relations::domain::relations::types::CellKind;

use ryo_relations::domain::relations::types::Relation;
