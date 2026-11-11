//! The `self_join_recogniser` service's cases for a request of two rules, through the compiled request.
use super::fixtures::fixture;
use crate::application::file_dag::service::prepare;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::file_program::service::compile_prepared;
use ryo_execution_domain::domain::file_program::types::ResolvedAlternative;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::self_join_recogniser::types::Decline;
use ryo_execution_store::outbound::derived::factory::semantics;
use ryo_values::domain::plan_parts::types::StepId;
use std::borrow::Cow;
use std::sync::Arc;

/// The second rule of the request: the first rule's steps up to the one it emits directly.
#[derive(Clone, Copy, Debug)]
struct SecondRule {
    emits: StepId,
}

/// What a recogniser resolved for one self join of the compiled request.
#[derive(Clone, Debug, PartialEq)]
enum Alternative {
    /// Nothing was resolved.
    Unresolved,
    /// An ordered existence the engine can run.
    Eligible,
    /// The self join stays a join, for this reason.
    Declined(Decline),
}

/// What the request answered before and after its regions were resolved.
#[derive(Debug, PartialEq)]
struct Resolution {
    /// How many rules had an ordered existence compiled before their regions were resolved.
    compiled_before_regions: usize,
    /// How many local shapes the first rule's ordered existence holds once resolved.
    first_rule_shapes: Option<usize>,
    /// What the compiled request resolved for each self join.
    alternatives: Cow<'static, [Alternative]>,
}

type Input = SecondRule;
type Value = Resolution;
type Failure = OperatingError;

const CASES: &[Case<Input, Value, Failure>] = &[Case {
    // The second rule observes the pair-multiplicity bag (step 2) directly, which makes the select's
    // publication bag-sensitive: the local shape is declined and never replaced.
    name: CaseName("request_binding_rejects_a_second_rules_bag_sensitive_terminal_select"),
    word: Word::Succeeds,
    input: || SecondRule { emits: StepId(2) },
    expect: Expect::Value(Resolution {
        compiled_before_regions: 0,
        first_rule_shapes: Some(1),
        alternatives: Cow::Borrowed(&[
            Alternative::Unresolved,
            Alternative::Unresolved,
            Alternative::Declined(Decline::ExtraConsumer),
            Alternative::Unresolved,
            Alternative::Unresolved,
        ]),
    }),
}];

/// Prepares and compiles the request of the first rule and the second.
fn subject(second: Input) -> Result<Value, Failure> {
    let first = fixture();
    let mut rule = first.clone();
    rule.steps.truncate(second.emits.0 as usize + 1);
    rule.emit.step = second.emits;
    let plans = [first, rule];
    let request = prepare(&plans, &[vec![], vec![]], &[], Arc::new(semantics()))?;
    let compiled_before_regions = request
        .ordered_existence
        .iter()
        .filter(|slot| slot.get().is_some())
        .count();
    let graph = compile_prepared(&plans, &request);
    Ok(Resolution {
        compiled_before_regions,
        first_rule_shapes: request.ordered_existence[0]
            .get()
            .map(|program| program.shapes.len()),
        alternatives: graph
            .ordered_existence
            .iter()
            .map(|alternative| match alternative {
                None => Alternative::Unresolved,
                Some(ResolvedAlternative::Eligible(_)) => Alternative::Eligible,
                Some(ResolvedAlternative::Declined(why)) => Alternative::Declined(*why),
            })
            .collect::<Vec<_>>()
            .into(),
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
