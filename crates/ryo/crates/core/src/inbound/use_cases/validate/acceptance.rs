//! The steps of the `validate` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};

use crate::Behavior;
use std::sync::Arc;

use ryo_core::inbound::use_cases::validate::interface::ValidateUseCase as _;
use ryo_core::inbound::use_cases::validate::types::ValidateRequest;
use ryo_core::inbound::use_cases::validate::use_case::ValidateUseCaseImpl;
use ryo_core::outbound::language::fixtures::fake::LanguageFake;
use ryo_core::outbound::language::interface::LanguagePort;
use ryo_core::outbound::query_language::fixtures::fake::{QueryLanguageFake, REFUSED};
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::hosted::types::{ColumnLabel, HostedColumn, HostedName, HostedRelation};
use ryo_values::domain::plan_parts::types::QueryText;

/// Names the scene the next `validate` step runs.
#[given(expr = "the validate input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Runs `validate` over the scene the input names, keeping what it answered: `valid` and `hosted` a query the fake finds correct,
/// without and with the host's `host.placed`; `kind` a query comparing a kind with a word no language has, with the fake language
/// linked or with `no language`; `refuse` a text the fake refuses.
#[when(expr = "validate runs")]
fn runs(world: &mut Behavior) {
    let input = world.input.as_str();
    let query_language = if input.starts_with("kind") {
        QueryLanguageFake::comparing_a_kind()
    } else {
        QueryLanguageFake::default()
    };
    let language: Arc<dyn LanguagePort> = if input.ends_with("no language") {
        Arc::new(ryo_core::outbound::language::factory::use_case(Vec::new()))
    } else {
        Arc::new(LanguageFake::default())
    };
    let hosted = if input == "hosted" {
        vec![HostedRelation {
            name: HostedName("host.placed".into()),
            columns: vec![HostedColumn {
                name: ColumnLabel("path".into()),
                kind: CellKind::File,
            }],
        }]
    } else {
        Vec::new()
    };
    let text = if input == "refuse" { REFUSED } else { input };
    let use_case = ValidateUseCaseImpl::new(Arc::new(query_language), language);
    world.answer = match use_case.validate(ValidateRequest {
        observations: Vec::new(),
        text: ryo_values::domain::program::types::QueryBundle::from(QueryText(text.into())),
        hosted,
    }) {
        Ok(report) => format!(
            "valid, reading [{}]",
            report
                .relations
                .iter()
                .map(|name| &*name.0)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Err(error) => error.to_string(),
    };
}

/// Asserts what the last `validate` step answered.
#[then(expr = "validate answers {string}")]
#[expect(
    clippy::needless_pass_by_value,
    reason = "cucumber parses a step's `{string}` into an owned `String` and passes it by value"
)]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected, "validate answered otherwise");
}
