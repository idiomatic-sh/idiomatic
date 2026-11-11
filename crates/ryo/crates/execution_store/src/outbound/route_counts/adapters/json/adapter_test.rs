//! The json adapter of the route_counts port: the core's contract rows run through it, then the cases specific to it.

use super::adapter::RouteCountsJsonAdapter;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::route_counts::fixtures::contract;
use crate::outbound::route_counts::interface::RouteCountsPort as _;
use crate::outbound::route_counts::types::{RouteField, RouteValue};
use std::borrow::Cow;

/// A route record the json adapter writes.
#[derive(Clone, Copy, Debug)]
enum Written {
    /// A text holding a quote and a line break.
    QuotedText,
    /// A record of a flag, an absent value and a list of counts.
    Record,
}

type Input = Written;
type Value = Cow<'static, str>;
type Failure = ();

/// The rows of what is specific to this adapter: what JSON it writes.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_quote_and_a_line_break_are_escaped"),
        word: Word::Succeeds,
        input: || Written::QuotedText,
        expect: Expect::Value(Cow::Borrowed("\"a\\\"b\\n\"")),
    },
    Case {
        name: CaseName("a_record_is_an_object_of_its_fields_in_order"),
        word: Word::Succeeds,
        input: || Written::Record,
        expect: Expect::Value(Cow::Borrowed(
            "{\"owned\":true,\"plan\":null,\"populations\":[3,4]}",
        )),
    },
];

/// The json the adapter writes for one input.
fn subject(input: Input) -> Result<Value, Failure> {
    let record = match input {
        Written::QuotedText => RouteValue::Text("a\"b\n".to_owned()),
        Written::Record => RouteValue::Record(vec![
            RouteField {
                key: "owned",
                value: RouteValue::Flag(true),
            },
            RouteField {
                key: "plan",
                value: RouteValue::Absent,
            },
            RouteField {
                key: "populations",
                value: RouteValue::List(vec![RouteValue::Count(3), RouteValue::Count(4)]),
            },
        ]),
    };
    Ok(Cow::Owned(RouteCountsJsonAdapter.document(&record).0))
}

#[test]
fn succeeds() {
    contract::check(&RouteCountsJsonAdapter);
    run(CASES, Word::Succeeds, subject);
}
