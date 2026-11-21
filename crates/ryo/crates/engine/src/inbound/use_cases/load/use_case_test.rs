//! What the `load` use case promises: an empty rule set loads, a pack reads back only for its own digest and engine, and a rule
//! the encoding refuses takes only itself, or the rules written after it when a count of the pack ran out, while the others
//! load, with the rules lowered twice at most.

use super::errors::LoadError;
use super::interface::LoadUseCase as _;
use super::service::sequenced;
use super::types::{LoadSource, LoadedRules};
use super::use_case::LoadUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::code::types::{MeasuredForm, ShapeChoice, UnitKey};
use crate::domain::findings::types::RuleColumns;
use crate::domain::lowered::types::{Branch, Emptiness, LoweredPack, LoweredRule};
use crate::domain::lowering::fixtures::{digests_of, named};
use crate::domain::lowering::types::{LoweredRules, SelectedRule, SelectedRules};
use crate::domain::packs::constants::VERSION_WIDTH;
use crate::domain::packs::service::encoded;
use crate::domain::packs::types::{PackBytes, PackDigest};
use crate::domain::programs::service::{accepted, unit_step};
use crate::domain::programs::types::Programs;
use crate::domain::statistics::types::{
    LearnStage, MatchedStatistics, MeasuredSizes, MeasuredUnit, OrderVerdict, Statistics,
    StatisticsBytes, UnitCounts,
};
use crate::domain::steps::types::{
    AnchorKinds, PlacedStep, Polarity, ProgramStep, Register, RegisterList, StepBody, StepMemo,
    StepProgram,
};
use ryo_relations::domain::relations::types::{KindBits, Relation};
use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
use ryo_values::domain::query_bindings::types::RuleBindings;
use ryo_values::domain::query_plans::types::{NodeDigest, NodeDigests};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::borrow::Cow;
use std::cell::Cell;

/// What a row loads.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// No rule.
    NoRules,
    /// A pack written for the digest asked.
    Pack,
    /// Three rules, the second of which the encoding cannot write: its own fault.
    OneFault,
    /// Three rules, the second of which runs a count of the pack out.
    OneCountOut,
    /// Three rules, the second and the third of which the encoding cannot write.
    TwoFaults,
    /// Three rules, loaded with statistics that name the units of the first and the third and a unit no program has.
    Measured,
    /// Three rules, loaded with the statistics of a first sample that measured no unit: not the same as no statistics.
    MeasuredNothing,
    /// Two rules of a plan of three steps, each given its steps' digests with its own unit's node second: where the loaded
    /// programs say each rule's unit, and a node that is no step of its plan, are in the rule's plan.
    UnitSteps,
    /// A pack written for another digest.
    OtherPack,
    /// A pack another engine version wrote.
    OtherVersion,
    /// Bytes that are no pack.
    NoPack,
}

/// What the encoding makes of a rule of the three.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Written {
    /// It is written.
    Whole,
    /// A second anchor stands after its first: the encoding's own fault.
    Faulty,
    /// A row of 65,536 registers: a count past its width.
    Wide,
}

type Input = Scenario;
/// The rules the loaded set runs, each rule refused with why, and how many times the rules were lowered.
type Value = Cow<'static, str>;
type Failure = LoadError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_empty_rule_set_loads"),
        word: Word::Succeeds,
        input: || Scenario::NoRules,
        expect: Expect::Value(Cow::Borrowed("runs []; refused []")),
    },
    Case {
        name: CaseName("a_pack_loads_for_its_digest"),
        word: Word::Succeeds,
        input: || Scenario::Pack,
        expect: Expect::Value(Cow::Borrowed("runs []; refused []")),
    },
    Case {
        name: CaseName("one_rules_fault_at_encoding_is_its_own_refusal_and_the_others_load"),
        word: Word::Succeeds,
        input: || Scenario::OneFault,
        expect: Expect::Value(Cow::Borrowed(
            "runs [0, 2]; refused [1: step PlanNode(0) cannot be evaluated]; lowered 2 times",
        )),
    },
    Case {
        name: CaseName(
            "a_count_of_the_pack_past_its_width_takes_the_rule_being_written_and_those_after_it_in_one_pass",
        ),
        word: Word::Succeeds,
        input: || Scenario::OneCountOut,
        expect: Expect::Value(Cow::Borrowed(
            "runs [0]; refused [1: more than 65,535 operands in one word; 2: more than 65,535 operands in one word]; lowered 2 times",
        )),
    },
    Case {
        name: CaseName("a_second_fault_loads_nothing_and_nothing_is_lowered_a_third_time"),
        word: Word::Succeeds,
        input: || Scenario::TwoFaults,
        expect: Expect::Value(Cow::Borrowed(
            "runs []; refused [0: step PlanNode(0) cannot be evaluated; 1: step PlanNode(0) cannot be evaluated; 2: step PlanNode(0) cannot be evaluated]; lowered 2 times",
        )),
    },
    Case {
        name: CaseName("a_load_counts_the_units_its_statistics_name"),
        word: Word::Succeeds,
        input: || Scenario::Measured,
        expect: Expect::Value(Cow::Borrowed(
            "runs [0, 1, 2]; refused []; statistics matched 2 units; lowered 1 times",
        )),
    },
    Case {
        name: CaseName("statistics_that_name_no_unit_match_nought_and_are_not_no_statistics"),
        word: Word::Succeeds,
        input: || Scenario::MeasuredNothing,
        expect: Expect::Value(Cow::Borrowed(
            "runs [0, 1, 2]; refused []; statistics matched 0 units; lowered 1 times",
        )),
    },
    Case {
        name: CaseName("a_units_plan_step_is_its_nodes_place_in_the_rules_own_plan"),
        word: Word::Succeeds,
        input: || Scenario::UnitSteps,
        expect: Expect::Value(Cow::Borrowed(
            "rule 0: its unit at step 1, a node of no step at step 2; rule 1: its unit at step 1, a node of no step at step 2",
        )),
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

/// The node of the rule of an ordinal: each rule's own.
fn node(rule: SelectedRuleOrdinal) -> NodeDigest {
    NodeDigest([u8::try_from(rule.0).unwrap_or(u8::MAX); 16])
}

/// Statistics that name the unit of each rule of `rules`.
fn naming<const N: usize>(rules: [u32; N]) -> Statistics {
    let unit = |rule| MeasuredUnit {
        key: UnitKey::Rule(node(SelectedRuleOrdinal(rule))),
        form: MeasuredForm::default(),
        shape: ShapeChoice::default(),
        order: OrderVerdict::default(),
        points: Vec::new(),
        counts: UnitCounts::default(),
    };
    Statistics {
        stage: LearnStage::First,
        sizes: MeasuredSizes::default(),
        units: rules.map(unit).into(),
    }
}

/// A lowered rule as the encoding makes `written` of it.
fn lowered_rule(rule: SelectedRuleOrdinal, written: Written) -> LoweredRule {
    let anchor = || ProgramStep::EachKind {
        relation: Relation::SyntaxTree,
        kinds: AnchorKinds(Box::new([KindBits(2)])),
        out: Register(0),
    };
    let placed = |step| PlacedStep { point: None, step };
    let row = RegisterList((0..=u32::from(u16::MAX)).map(Register).collect());
    let wide = ProgramStep::Exists {
        polarity: Polarity::Present,
        body: StepBody::Inline(StepProgram(Box::new([placed(ProgramStep::Yield { row })]))),
        memo: StepMemo::None,
    };
    let steps = match written {
        Written::Whole => vec![placed(anchor())],
        Written::Faulty => vec![placed(anchor()), placed(anchor())],
        Written::Wide => vec![placed(anchor()), placed(wide)],
    };
    LoweredRule {
        rule,
        node: node(rule),
        branches: vec![Branch {
            emptiness: Emptiness::Unknown,
            program: StepProgram(steps.into()),
        }],
        columns: RuleColumns::default(),
    }
}

/// A load said: the rules it runs and each rule it refused.
fn said(loaded: &LoadedRules) -> String {
    let refused: Vec<String> = loaded
        .refused
        .iter()
        .map(|rule| format!("{:?}: {}", rule.rule.0, rule.refusal))
        .collect();
    let runs: Vec<u32> = accepted(&loaded.programs)
        .iter()
        .map(|rule| rule.0)
        .collect();
    format!("runs {runs:?}; refused [{}]", refused.join("; "))
}

/// Loads three copies of the tree rule through a lowering that lowers each as `written` says of its ordinal.
fn sequence(written: [Written; 3], statistics: Option<&Statistics>) -> Value {
    let plan = named("one tree rule");
    let digests = digests_of(&plan, RuleBindings(&[]));
    let rules: Vec<SelectedRule<'_>> = (0..3)
        .map(|rule| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan: &plan,
            bindings: RuleBindings(&[]),
            digests: &digests,
        })
        .collect();
    let lowerings = Cell::new(0_usize);
    let lower = |selected: SelectedRules<'_>| {
        lowerings.set(lowerings.get().saturating_add(1));
        let of = |rule: &SelectedRule<'_>| {
            let place = usize::try_from(rule.rule.0).unwrap_or(usize::MAX);
            lowered_rule(
                rule.rule,
                written.get(place).copied().unwrap_or(Written::Whole),
            )
        };
        LoweredRules {
            pack: LoweredPack {
                rules: selected.0.iter().map(of).collect(),
                ..LoweredPack::default()
            },
            refused: Vec::new(),
        }
    };
    let loaded = sequenced(SelectedRules(&rules), statistics, &lower);
    let matched = match loaded.matched {
        MatchedStatistics::Units(matched) => format!("; statistics matched {} units", matched.0),
        MatchedStatistics::Absent => String::new(),
    };
    Cow::Owned(format!(
        "{}{matched}; lowered {} times",
        said(&loaded),
        lowerings.get()
    ))
}

/// Loads two rules of a plan of three steps, each with the digests a host would give it but its own unit's node as its second
/// step's, and says where the programs put each rule's unit and a node of no step of its plan.
fn unit_steps() -> Value {
    let plan = named("two orders of one rule");
    let digests: Vec<NodeDigests> = (0..2)
        .map(|rule| {
            let mut digests = digests_of(&plan, RuleBindings(&[]));
            if let Some(second) = digests.0.get_mut(1) {
                *second = node(SelectedRuleOrdinal(rule));
            }
            digests
        })
        .collect();
    let rules: Vec<SelectedRule<'_>> = digests
        .iter()
        .zip(0..)
        .map(|(digests, rule)| SelectedRule {
            rule: SelectedRuleOrdinal(rule),
            plan: &plan,
            bindings: RuleBindings(&[]),
            digests,
        })
        .collect();
    let lower = |selected: SelectedRules<'_>| LoweredRules {
        pack: LoweredPack {
            rules: selected
                .0
                .iter()
                .map(|rule| lowered_rule(rule.rule, Written::Whole))
                .collect(),
            ..LoweredPack::default()
        },
        refused: Vec::new(),
    };
    let loaded = sequenced(SelectedRules(&rules), None, &lower);
    let said: Vec<String> = (0..2)
        .map(SelectedRuleOrdinal)
        .map(|rule| {
            let own = unit_step(&loaded.programs, rule, &UnitKey::Rule(node(rule)));
            let other = unit_step(&loaded.programs, rule, &UnitKey::Rule(NodeDigest([77; 16])));
            format!(
                "rule {}: its unit at step {}, a node of no step at step {}",
                rule.0, own.0, other.0
            )
        })
        .collect();
    Cow::Owned(said.join("; "))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let kinds = KindVocabulary {
        language: LanguageName("walking".into()),
        kinds: Vec::new(),
        fields: Vec::new(),
        fixed: Vec::new(),
    };
    let pack = encoded(&Programs::default(), PackDigest([1; 32]));
    let mut other = pack.0.clone();
    if let Some(byte) = other.get_mut(VERSION_WIDTH.0) {
        *byte = byte.wrapping_add(1);
    }
    let source = match scenario {
        Scenario::OneFault => {
            return Ok(sequence(
                [Written::Whole, Written::Faulty, Written::Whole],
                None,
            ));
        }
        Scenario::OneCountOut => {
            return Ok(sequence(
                [Written::Whole, Written::Wide, Written::Whole],
                None,
            ));
        }
        Scenario::TwoFaults => {
            return Ok(sequence(
                [Written::Whole, Written::Faulty, Written::Faulty],
                None,
            ));
        }
        Scenario::Measured => return Ok(sequence([Written::Whole; 3], Some(&naming([0, 2, 9])))),
        Scenario::MeasuredNothing => return Ok(sequence([Written::Whole; 3], Some(&naming([])))),
        Scenario::UnitSteps => return Ok(unit_steps()),
        Scenario::NoRules => LoadSource::Plans(SelectedRules(&[]), StatisticsBytes(&[])),
        Scenario::Pack => LoadSource::Pack(PackBytes(&pack.0), PackDigest([1; 32])),
        Scenario::OtherPack => LoadSource::Pack(PackBytes(&pack.0), PackDigest([2; 32])),
        Scenario::OtherVersion => LoadSource::Pack(PackBytes(&other), PackDigest([1; 32])),
        Scenario::NoPack => LoadSource::Pack(PackBytes(b"no"), PackDigest([1; 32])),
    };
    LoadUseCaseImpl
        .load(source, &kinds)
        .map(|loaded| Cow::Owned(said(&loaded)))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
