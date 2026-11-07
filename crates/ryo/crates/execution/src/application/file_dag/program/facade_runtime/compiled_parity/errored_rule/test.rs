//! Preparing two written rules, of which one has a step beyond its operator contract, succeeds, and only that rule's metadata
//! refuses: its emit root and its named-output sources carry the error, while its sibling's roots stay usable. Each rule's
//! written text is compiled through the query-language port and its parameters bound from its written spec; the provider
//! hands over the plans (RyoQL refuses both authoring errors itself, so a provider is the author that can hand them over).
use super::super::*;
use crate::outbound::query_language::{
    errors::QueryLanguageError, fixtures::fake::QueryLanguageFake, interface::QueryLanguagePort,
};
use ryo_values::domain::query_tokens::types::{QueryBundle, QuerySourceText};
use ryo_values::domain::runs::types::RuleId;
use crate::application::file_dag::program::preparation::service::prepare;
use crate::domain::coverage::types::RuleIndex;
use crate::domain::executable_program::types::InputSource;
use ryo_values::domain::observations::types::{
    ObservationColumn, ObservationColumnName, ObservationName, PlanObservation,
};
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::plans::types::PlanStep;

/// The query-language provider the rules are written against: each written text compiles to the plan handed over for it.
struct Provider(Vec<(&'static str, QueryLanguageFake)>);

fn written<'a>(provider: &'a Provider, text: &QueryBundle) -> &'a QueryLanguageFake {
    provider
        .0
        .iter()
        .find(|(written, _)| *written == &*text.root.text)
        .map(|(_, fake)| fake)
        .expect("every written rule has a text the provider compiles")
}

impl QueryLanguagePort for Provider {
    fn validate(
        &self,
        bundle: &QueryBundle,
        hosted: &[ryo_values::domain::hosted::types::HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ryo_values::domain::descriptions::types::ValidateReport, QueryLanguageError> {
        written(self, bundle).validate(bundle, hosted, observations)
    }
    fn compile(
        &self,
        bundle: &QueryBundle,
        hosted: &[ryo_values::domain::hosted::types::HostedRelation],
        observations: &[ryo_values::domain::observations::types::BoundObservationInput],
    ) -> Result<ryo_values::domain::plans::types::Plan, QueryLanguageError> {
        written(self, bundle).compile(bundle, hosted, observations)
    }
}

/// A rule written as a user writes it: an id and its query text.
fn written_rule(id: &str, text: &str) -> ryo_values::domain::rule_requests::types::RuleSpec {
    let mut spec = rule();
    spec.id = RuleId(id.into());
    spec.query.root.text = QuerySourceText(text.into());
    spec
}

#[test]
fn only_the_rule_with_an_errored_step_is_refused_on_its_roots() {
    let sound = plan(Scenario::Words);
    let mut errored = plan(Scenario::Words);
    // The step after the emit step names a column the select does not keep: the origin is errored, the emit step is sound.
    errored.steps.push(PlanStep {
        operation: Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![ColumnIndex(99)],
            distinct: ryo_values::domain::plan_parts::types::RepeatedRows::Kept,
        },
        scope: StepScope::PerFile,
    });
    errored.observations = vec![PlanObservation {
        name: ObservationName("kept".into()),
        source: PlanInput::Step(StepId(0)),
        columns: vec![
            ObservationColumn {
                name: ObservationColumnName("node".into()),
                kind: kinds(Relation::SyntaxTree)[0],
            },
            ObservationColumn {
                name: ObservationColumnName("kind".into()),
                kind: kinds(Relation::SyntaxTree)[1],
            },
        ],
        scope: StepScope::PerFile,
    }];
    let rules = [
        written_rule("sound", "sound rule"),
        written_rule("errored", "errored rule"),
    ];
    let provider = Provider(vec![
        (
            "sound rule",
            QueryLanguageFake {
                plan: sound,
                ..QueryLanguageFake::default()
            },
        ),
        (
            "errored rule",
            QueryLanguageFake {
                plan: errored,
                ..QueryLanguageFake::default()
            },
        ),
    ]);
    let plans = rules
        .iter()
        .map(|rule| provider.compile(&rule.query, &[], &[]))
        .collect::<Result<Vec<_>, _>>()
        .expect("both written rules compile");
    let bindings = plans
        .iter()
        .zip(&rules)
        .map(|(plan, rule)| ryo_values::domain::rule_requests::service::bind(plan, rule))
        .collect::<Result<Vec<_>, _>>()
        .expect("both written rules bind their parameters");
    let compiled = Compiled {
        schedule: RuleSchedule::Independent,
        observations: vec![vec![], vec![]],
        plans,
        bindings,
    };
    let derived = crate::application::file_dag::fixtures::program_derived_port();
    let prepared = prepare(
        compiled,
        &rules,
        &[Digest([0; 16]), Digest([1; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .expect("an errored rule does not refuse the request: it is refused on its own roots");
    let sibling = prepared
        .rule_metadata(RuleIndex(0))
        .expect("the sound rule has metadata");
    assert!(
        sibling.emit_root.is_ok(),
        "the sibling rule keeps its emit root"
    );
    let refused = prepared
        .rule_metadata(RuleIndex(1))
        .expect("the errored rule has metadata");
    let error = refused
        .emit_root
        .clone()
        .expect_err("the errored rule's emit root is its error, though its emit step is sound");
    assert_eq!(refused.observations.len(), 1, "the named output is kept");
    for output in refused.observations.iter() {
        assert_eq!(
            output.source,
            Err::<InputSource, _>(error.clone()),
            "the named output's source is the same error"
        );
    }
}
