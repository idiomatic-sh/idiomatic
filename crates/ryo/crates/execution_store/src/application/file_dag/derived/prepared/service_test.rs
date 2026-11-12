//! Behavioral proof of `prepared`: each rule of a request answers the relations it reads and whether it may walk a
//! graph, and a rule the request does not hold is not run.
use super::service::RequestDerived;
use super::types::RuleDerivedNeeds;
use crate::application::file_dag::derived::owner::cache::types::GraphAdmission;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_relations::domain::relations::types::Relation;
use std::borrow::Cow;

/// The rule of the request below that is asked for.
type Input = usize;
/// The relations the rule reads, and whether it may request graph admission.
type Value = (Cow<'static, [Relation]>, GraphAdmission);
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_rule_reads_its_own_relations_and_walks_no_graph"),
        word: Word::Succeeds,
        input: || 0,
        expect: Expect::Value((
            Cow::Borrowed(&[Relation::Function]),
            GraphAdmission::Disabled,
        )),
    },
    Case {
        name: CaseName("a_graph_rule_may_request_graph_admission"),
        word: Word::Succeeds,
        input: || 1,
        expect: Expect::Value((
            Cow::Borrowed(&[Relation::Function, Relation::LanguageTarget]),
            GraphAdmission::Enabled,
        )),
    },
    Case {
        name: CaseName("a_rule_beyond_the_request_is_not_run"),
        word: Word::Refuses,
        input: || 2,
        expect: Expect::Error(OperatingError::NotRun),
    },
];

/// A request of two rules: the first reads functions, the second functions and targets and walks a graph.
fn subject(rule: Input) -> Result<Value, Failure> {
    let request = RequestDerived::new(Box::new([
        RuleDerivedNeeds {
            relations: Box::new([Relation::Function]),
            graph: GraphAdmission::Disabled,
        },
        RuleDerivedNeeds {
            relations: Box::new([Relation::Function, Relation::LanguageTarget]),
            graph: GraphAdmission::Enabled,
        },
    ]));
    let relations = Cow::Owned(request.relations_for(RuleIndex(rule))?.to_vec());
    Ok((relations, request.graph_for(RuleIndex(rule))?))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
