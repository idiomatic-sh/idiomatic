//! The steps of the `load` feature: each a cucumber expression over the world, one to six lines, asserting the part's own results.

use cucumber::{given, then, when};
use ryo_engine::domain::lowering::fixtures::named;
use ryo_engine::domain::lowering::types::{SelectedRule, SelectedRules};
use ryo_engine::domain::packs::constants::VERSION_WIDTH;
use ryo_engine::domain::packs::service::encoded;
use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
use ryo_engine::domain::programs::service::accepted;
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::interface::LoadUseCase as _;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_engine::inbound::use_cases::load::use_case::LoadUseCaseImpl;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

use crate::Behavior;

/// The scene that names a rule set of no rule.
const NO_RULES: &str = "no rules";

/// The scene whose pack another engine version wrote.
const OTHER_VERSION: &str = "other version";

/// The digest the scenarios' packs are written for.
const DIGEST: PackDigest = PackDigest([5; 32]);

/// The kinds of the fixture language.
fn kinds() -> KindVocabulary {
    KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
    }
}

/// The rule set a scene names, lowered: no rule, or rule 0 with the scene's plan.
pub(crate) fn lowered(scene: &str) -> Result<Programs, LoadError> {
    let plan = named(scene);
    let rules = [SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan: &plan,
    }];
    let selected = if scene == NO_RULES {
        &rules[..0]
    } else {
        &rules[..]
    };
    LoadUseCaseImpl.load(LoadSource::Plans(SelectedRules(selected)), &kinds())
}

/// Names the scene the next `load` step runs.
#[given(expr = "the load input {string}")]
fn given_input(world: &mut Behavior, value: String) {
    world.input = value;
}

/// Lowers the scene's rule set, answering how many rules it runs or why it refused.
#[when(expr = "load runs")]
fn runs(world: &mut Behavior) {
    world.answer = match lowered(&world.input) {
        Ok(programs) => format!("loaded {} rules", accepted(&programs).len()),
        Err(error) => error.to_string(),
    };
}

/// Lowers the scene's rule set, encodes it and loads the pack back, answering how many rules both run or why one refused.
#[when(expr = "load runs through a pack")]
fn runs_through_a_pack(world: &mut Behavior) {
    let programs = if world.input == OTHER_VERSION {
        Ok(Programs::default())
    } else {
        lowered(&world.input)
    };
    world.answer = match programs {
        Ok(programs) => {
            let mut bytes = encoded(&programs, DIGEST).0;
            if world.input == OTHER_VERSION
                && let Some(format) = bytes.get_mut(VERSION_WIDTH.0)
            {
                *format = format.wrapping_add(1);
            }
            match LoadUseCaseImpl.load(LoadSource::Pack(PackBytes(&bytes), DIGEST), &kinds()) {
                Ok(read) if accepted(&read) == accepted(&programs) => {
                    format!("pack and lowering run {} rules", accepted(&read).len())
                }
                Ok(read) => format!(
                    "pack runs {:?}, lowering {:?}",
                    accepted(&read),
                    accepted(&programs)
                ),
                Err(error) => error.to_string(),
            }
        }
        Err(error) => error.to_string(),
    };
}

/// Asserts what `load` answered.
#[then(expr = "load answers {string}")]
fn answers(world: &mut Behavior, expected: String) {
    assert_eq!(world.answer, expected);
}
