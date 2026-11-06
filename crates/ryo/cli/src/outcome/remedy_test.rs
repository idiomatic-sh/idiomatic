//! Proves what the user does next about each error a command can end with.

use super::{Next, Remedy as _};
use crate::resources::operator::describe::schemas::DescribeSchemaError as DescribeOperatorRefusal;
use crate::resources::query::run::schemas::RunSchemaError as RunQueryRefusal;
use crate::resources::query::validate::schemas::ValidateSchemaError as ValidateQueryRefusal;
use crate::resources::relation::describe::schemas::DescribeSchemaError as DescribeRelationRefusal;

/// A command line that moves the user on is printed as the command to run.
#[test]
fn next_run_succeeds_as_the_command_to_run() {
    assert_eq!(
        Next::Run("ryo relation list".to_owned()).to_string(),
        "run `ryo relation list`"
    );
}

/// What to change is printed as the instruction it is.
#[test]
fn next_fix_succeeds_as_the_instruction() {
    assert_eq!(
        Next::Fix("pass the query with `--text`".to_owned()).to_string(),
        "pass the query with `--text`"
    );
}

/// A bug is printed with where to report it.
#[test]
fn next_report_succeeds_with_where_to_report() {
    let said = Next::Report.to_string();
    assert!(said.contains("this is a bug"), "{said}");
    assert!(said.contains("/issues/new"), "{said}");
}

/// A blank query text is fixed by passing the query with `--text`, whichever command refused it.
#[test]
fn a_blank_query_refuses_with_the_text_flag() {
    let fix = Next::Fix("pass the query with `--text`".to_owned());
    assert_eq!(RunQueryRefusal::BlankText.next(), fix);
    assert_eq!(ValidateQueryRefusal::BlankText.next(), fix);
}

/// Only a refused kind is told to correct the kind: a query that cannot be compiled is told to correct the text.
#[test]
fn only_a_kind_refusal_is_told_to_correct_the_kind() {
    use ryo_core::inbound::use_cases::validate::errors::ValidateError;
    use ryo_relations::domain::relations::types::{Offset, Span};
    use ryo_values::domain::plan_parts::types::{RefusalMessage, RefusedKind, RefusedQuery};
    assert_eq!(
        ValidateError::UnknownKind {
            reason: "no such kind".to_owned(),
            origin: ryo_values::domain::program::types::Attribution::Unlocated,
        }
        .next(),
        Next::Fix("correct the kind the query text compares with".to_owned())
    );
    assert_eq!(
        ValidateError::Uncompilable {
            error: RefusedQuery {
                kind: RefusedKind::Invalid,
                message: RefusalMessage("cannot compile".to_owned()),
                span: Span {
                    start: Offset(0),
                    end: Offset(0),
                },
                origin: ryo_values::domain::program::types::Attribution::Unlocated,
            },
        }
        .next(),
        Next::Fix("correct the query text where the message says".to_owned())
    );
}

/// A blank relation name is fixed by listing the relations.
#[test]
fn a_blank_relation_name_refuses_with_the_relation_list() {
    assert_eq!(
        DescribeRelationRefusal::BlankName.next(),
        Next::Run("ryo relation list".to_owned())
    );
}

/// A blank operator name is fixed by listing the operators.
#[test]
fn a_blank_operator_name_refuses_with_the_operator_list() {
    assert_eq!(
        DescribeOperatorRefusal::BlankName.next(),
        Next::Run("ryo operator list".to_owned())
    );
}

/// A word that names no kind is fixed in the query text; the one query's failure is said as its own, and one of several names the
/// query by its place.
#[test]
fn an_unknown_kind_refuses_naming_the_query_only_among_several() {
    use ryo_core::inbound::use_cases::lint::errors::LintError;
    let error = LintError::UnknownKind {
        id: "2".to_owned(),
        reason: "kind `x` names no kind of fake".to_owned(),
    };
    let alone = RunQueryRefusal::Failed(error.clone());
    let among = RunQueryRefusal::FailedAmong(error);
    assert_eq!(alone.to_string(), "kind `x` names no kind of fake");
    assert_eq!(
        among.to_string(),
        "rule `2`: kind `x` names no kind of fake"
    );
    assert!(matches!(alone.next(), Next::Fix(fix) if fix.contains("query validate")));
}
