//! What the `load` use case promises: an empty rule set loads, a rule the lowering cannot express is refused by name, and a pack
//! reads back only for its own digest and engine.

use super::errors::LoadError;
use super::interface::LoadUseCase as _;
use super::types::LoadSource;
use super::use_case::LoadUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::fixtures::named;
use crate::domain::lowering::types::{PlanNode, PlanShape, SelectedRule, SelectedRules};
use crate::domain::packs::constants::VERSION_WIDTH;
use crate::domain::packs::service::encoded;
use crate::domain::packs::types::{PackBytes, PackDigest};
use crate::domain::programs::service::accepted;
use crate::domain::programs::types::Programs;
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;

/// What a row loads.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// No rule.
    NoRules,
    /// One rule whose plan the lowering does not express yet.
    OneRule,
    /// A pack written for the digest asked.
    Pack,
    /// A pack written for another digest.
    OtherPack,
    /// A pack another engine version wrote.
    OtherVersion,
    /// Bytes that are no pack.
    NoPack,
}

type Input = Scenario;
/// How many rules the loaded set runs.
type Value = usize;
type Failure = LoadError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_rule_set_loads"),
        word: Word::Succeeds,
        input: || Scenario::NoRules,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_pack_loads_for_its_digest"),
        word: Word::Succeeds,
        input: || Scenario::Pack,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_rule_the_lowering_cannot_express_is_refused_by_name"),
        word: Word::Refuses,
        input: || Scenario::OneRule,
        expect: Expect::Error(LoadError::Unlowerable {
            rule: SelectedRuleOrdinal(0),
            shape: PlanShape::Step(PlanNode(0)),
        }),
    },
    Case {
        name: CaseName("a_pack_for_other_rules_is_refused"),
        word: Word::Refuses,
        input: || Scenario::OtherPack,
        expect: Expect::Error(LoadError::PackDigest),
    },
    Case {
        name: CaseName("a_pack_of_another_engine_version_is_refused"),
        word: Word::Refuses,
        input: || Scenario::OtherVersion,
        expect: Expect::Error(LoadError::PackVersion),
    },
    Case {
        name: CaseName("bytes_that_are_no_pack_are_refused"),
        word: Word::Refuses,
        input: || Scenario::NoPack,
        expect: Expect::Error(LoadError::PackCorrupt),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let plan = named("one tree rule");
    let rules = [SelectedRule {
        rule: SelectedRuleOrdinal(0),
        plan: &plan,
    }];
    let pack = encoded(&Programs::default(), PackDigest([1; 32]));
    let mut other = pack.0.clone();
    if let Some(byte) = other.get_mut(VERSION_WIDTH.0) {
        *byte = byte.wrapping_add(1);
    }
    let source = match scenario {
        Scenario::NoRules => LoadSource::Plans(SelectedRules(&[])),
        Scenario::OneRule => LoadSource::Plans(SelectedRules(&rules)),
        Scenario::Pack => LoadSource::Pack(PackBytes(&pack.0), PackDigest([1; 32])),
        Scenario::OtherPack => LoadSource::Pack(PackBytes(&pack.0), PackDigest([2; 32])),
        Scenario::OtherVersion => LoadSource::Pack(PackBytes(&other), PackDigest([1; 32])),
        Scenario::NoPack => LoadSource::Pack(PackBytes(b"no"), PackDigest([1; 32])),
    };
    LoadUseCaseImpl
        .load(source, &kinds)
        .map(|programs| accepted(&programs).len())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
