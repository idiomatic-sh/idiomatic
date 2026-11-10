//! The compile feature enters the real public rule compilation.
use crate::Behavior;
use cucumber::{given, then, when};
use ryo_execution::inbound::use_cases::compile_rules::interface::CompileRulesUseCase;
use ryo_execution::inbound::use_cases::compile_rules::types::CompileRequest;
use ryo_execution::inbound::use_cases::compile_rules::use_case::CompileRulesUseCaseImpl;
use ryo_values::domain::query_tokens::types::{QueryBundle, QueryDocumentSource, QuerySourceId};
use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
use ryo_values::domain::runs::types::{RuleApplicability, RuleId};
use std::sync::Arc;

#[given(expr = "compile rules {string}")]
fn rules(world: &mut Behavior, input: String) {
    world.input = input;
}

/// A rule `id` whose query the fake compiler compiles to its plan.
fn rule(id: &str) -> RuleSpec {
    RuleSpec {
        id: RuleId(id.into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(id.into()),
                text: "".into(),
            },
            modules: Default::default(),
        },
        applicability: RuleApplicability::Always,
        given: Vec::new(),
        observations: Vec::new(),
        spelled: RuleParameters("".into()),
        include: Vec::new(),
        exclude: Vec::new(),
    }
}

#[when(expr = "compilation runs")]
fn runs(world: &mut Behavior) {
    let compile = CompileRulesUseCaseImpl::new(
        Arc::new(
            ryo_execution::outbound::query_language::fixtures::fake::QueryLanguageFake::default(),
        ),
        Arc::new(ryo_execution_runtime::outbound::digest::factory::blake3()),
    );
    let rules = if world.input == "repeated" {
        vec![rule("a"), rule("a")]
    } else {
        vec![rule("a"), rule("b")]
    };
    let compiled = compile.compile_rules(CompileRequest {
        rules: &rules,
        hosted: &[],
        vocabularies: &ryo_values::domain::descriptions::types::KindVocabularies(Vec::new()),
    });
    world.answer = match compiled {
        Ok(_) => "compiled",
        Err(_) => "refused",
    }
    .to_owned();
}

#[then(expr = "compilation answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
