//! The pack load's cases, over the engine port's fake and the engine itself: a first run lowers and writes the pack; the same rules
//! read it back and lower nothing; a changed rule, another kind vocabulary or a pack that does not decode lowers again; and the
//! harness keeps the store's packs under the old engine, keeps the rule set's pack when `both` compares nothing, and stops a run
//! the new engine cannot finish.

use super::service::{harnessed, harnessed_in, loaded, pack_key};
use super::types::{Harness, HarnessLoad, HarnessRequest, LoadRequest};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::digest::factory::blake3;
use crate::outbound::engine::factory::use_case;
use crate::outbound::engine::fixtures::fake::EngineFake;
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::lowering::fixtures::named;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::spike::{EngineMode, engine_mode};
use ryo_relations::domain::relations::types::Bytes;
use ryo_values::domain::descriptions::types::{
    KindVocabularies, KindVocabulary, KindWord, LanguageName,
};
use ryo_values::domain::plans::types::Plan;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::StoredPack;
use std::borrow::Cow;
use std::sync::atomic::Ordering;

/// What a row does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// Three rules, one the fake accepts, with nothing stored.
    FirstRun,
    /// The same rules, with the first run's pack stored.
    SameRules,
    /// One rule's digest changed since the pack was stored.
    ChangedRule,
    /// A pack stored under the rules' key whose bytes do not decode.
    BrokenPack,
    /// No rule, through the engine itself: a first run, then a second with its pack.
    EmptyRuleSet,
    /// The old engine, with a pack stored.
    OldEngine,
    /// `both`, with one rule the fake refuses.
    BothNothingAccepted,
    /// `both`, with two languages linked and the second named.
    NamedLanguage,
    /// `new`, with a rule the fake accepts.
    NewEngine,
    /// `both`, with a rule the fake accepts.
    BothAccepted,
    /// `both`, at an edge that composed no engine.
    NoEngine,
    /// `both`, with two languages linked and none named: a rule set for each.
    TwoLanguages,
    /// `both`, with a named language that is not linked.
    NamedNotLinked,
}

type Input = Scenario;
/// What the load or the harness answered, said in one line.
type Value = Cow<'static, str>;
/// Why the harness stopped the run.
type Failure = Cow<'static, str>;

/// What a load that lowers the three rules says: one accepted, one the fake cannot lower, one over budget; each lowered alone,
/// the accepted one once more; no pack read.
const COLD: &str = "Lowering(Refusals { unlowerable: Count(1), over_budget: Count(1) }); accepted [0]; lowerings 4; packs read 0";

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_first_run_lowers_each_rule_and_writes_the_pack"),
        word: Word::Succeeds,
        input: || Scenario::FirstRun,
        expect: Expect::Value(Cow::Borrowed(COLD)),
    },
    Case {
        name: CaseName("the_same_rules_read_the_stored_pack_and_lower_nothing"),
        word: Word::Succeeds,
        input: || Scenario::SameRules,
        expect: Expect::Value(Cow::Borrowed(
            "Pack; accepted [0]; lowerings 0; packs read 1",
        )),
    },
    Case {
        name: CaseName("a_changed_rule_lowers_again"),
        word: Word::Succeeds,
        input: || Scenario::ChangedRule,
        expect: Expect::Value(Cow::Borrowed(COLD)),
    },
    Case {
        name: CaseName("a_stored_pack_that_does_not_decode_is_lowered_again"),
        word: Word::Succeeds,
        input: || Scenario::BrokenPack,
        expect: Expect::Value(Cow::Borrowed(
            "Lowering(Refusals { unlowerable: Count(1), over_budget: Count(1) }); accepted [0]; lowerings 4; packs read 1",
        )),
    },
    Case {
        name: CaseName("the_empty_rule_set_round_trips_through_the_engines_pack"),
        word: Word::Succeeds,
        input: || Scenario::EmptyRuleSet,
        expect: Expect::Value(Cow::Borrowed("Pack; accepted []; the pack kept as it was")),
    },
    Case {
        name: CaseName("the_old_engine_loads_nothing"),
        word: Word::Succeeds,
        input: || Scenario::OldEngine,
        expect: Expect::Value(Cow::Borrowed("[]; nothing loaded")),
    },
    Case {
        name: CaseName("both_with_no_accepted_rule_keeps_the_rule_sets_pack"),
        word: Word::Succeeds,
        input: || Scenario::BothNothingAccepted,
        expect: Expect::Value(Cow::Borrowed("[\"words\"]; the rule set's pack")),
    },
    Case {
        name: CaseName("the_named_language_is_the_one_lowered_against"),
        word: Word::Succeeds,
        input: || Scenario::NamedLanguage,
        expect: Expect::Value(Cow::Borrowed("[\"signs\"]; the rule set's pack")),
    },
    Case {
        name: CaseName("every_linked_language_has_a_rule_set_of_its_own"),
        word: Word::Succeeds,
        input: || Scenario::TwoLanguages,
        expect: Expect::Value(Cow::Borrowed("[\"words\", \"signs\"]; the rule set's pack")),
    },
    Case {
        name: CaseName("new_stops_a_run_no_linked_language_walks"),
        word: Word::Refuses,
        input: || Scenario::NewEngine,
        expect: Expect::Error(Cow::Borrowed(
            "engine harness: RYO_ENGINE=new: the new engine accepts 1 rules and runs no file yet (no linked language walks its files: the link holds parse and project only)",
        )),
    },
    Case {
        name: CaseName("both_with_an_accepted_rule_stops_a_run_no_linked_language_walks"),
        word: Word::Refuses,
        input: || Scenario::BothAccepted,
        expect: Expect::Error(Cow::Borrowed(
            "engine harness: RYO_ENGINE=both: the new engine accepts 1 rules and runs no file yet (no linked language walks its files: the link holds parse and project only)",
        )),
    },
    Case {
        name: CaseName("an_edge_that_composed_no_engine_stops_the_run"),
        word: Word::Refuses,
        input: || Scenario::NoEngine,
        expect: Expect::Error(Cow::Borrowed(
            "engine harness: RYO_ENGINE is set and this edge composed no engine port",
        )),
    },
    Case {
        name: CaseName("a_named_language_that_is_not_linked_stops_the_run"),
        word: Word::Refuses,
        input: || Scenario::NamedNotLinked,
        expect: Expect::Error(Cow::Borrowed(
            "engine harness: RYO_ENGINE lowers the rules against each linked language: none is linked, or the one the request names is not",
        )),
    },
];

/// The plans of the lowering fixture's scenes, in order.
fn plans(scenes: &[&str]) -> Vec<Plan> {
    scenes.iter().map(|scene| named(scene)).collect()
}

/// A digest of each seed.
fn digests(seeds: &[u8]) -> Vec<Digest> {
    seeds.iter().map(|seed| Digest([*seed; 16])).collect()
}

/// A language of one kind.
fn kinds(language: &str, kind: &str) -> KindVocabulary {
    KindVocabulary {
        language: LanguageName(language.into()),
        kinds: vec![KindWord(kind.into())],
    }
}

/// The three rules of the load cases: the fake accepts the first, cannot lower the second, and finds the third over budget.
fn three_rules() -> Vec<Plan> {
    plans(&[
        "one tree rule",
        "unbound correlated sub-plan",
        "program over budget",
    ])
}

/// What a load through a fake says: where the rule set came from, its rules, and what the fake was asked.
fn said(load: &HarnessLoad, fake: &EngineFake) -> Value {
    let accepted: Vec<u32> = load.accepted.iter().map(|rule| rule.0).collect();
    let (lowered, unpacked) = (
        fake.lowered.load(Ordering::Relaxed),
        fake.unpacked.load(Ordering::Relaxed),
    );
    Cow::Owned(format!(
        "{:?}; accepted {accepted:?}; lowerings {lowered}; packs read {unpacked}",
        load.from
    ))
}

/// A load of the three rules through a new fake, under `seeds` and `kind`, with `stored` in the store.
fn load(seeds: &[u8], kind: &str, stored: &[StoredPack]) -> (HarnessLoad, EngineFake) {
    let (fake, kinds, rules) = (EngineFake::default(), kinds("words", kind), three_rules());
    let request = LoadRequest {
        plans: &rules.iter().collect::<Vec<&Plan>>(),
        digests: &digests(seeds),
        kinds: &kinds,
        stored,
    };
    (loaded(&fake, &blake3(), request), fake)
}

/// The harness in `mode` over `engine`, the rules of `scenes` and the languages `linked`.
fn harness(
    mode: EngineMode,
    engine: Option<&dyn EnginePort>,
    scenes: &[&str],
    linked: Vec<KindVocabulary>,
    named: Option<&LanguageName>,
) -> Result<Vec<HarnessLoad>, Failure> {
    let (vocabularies, rules, digest) = (KindVocabularies(linked), plans(scenes), blake3());
    let request = HarnessRequest {
        plans: &rules.iter().collect::<Vec<&Plan>>(),
        digests: &digests(&[1]),
        stored: &[],
    };
    let harness = engine.map(|engine| Harness {
        engine,
        digest: &digest,
        vocabularies: &vocabularies,
        named,
    });
    harnessed_in(mode, harness, request).map_err(|error| Cow::Owned(error.to_string()))
}

/// What the harness loaded: each load's language, and whether the first load's pack is `expected`.
fn kept(loads: &[HarnessLoad], expected: &StoredPack) -> Value {
    let languages: Vec<&str> = loads.iter().map(|load| &*load.language.0).collect();
    let pack = match loads.first() {
        Some(load) if load.pack == *expected => "the rule set's pack",
        Some(_) => "another pack",
        None => "nothing loaded",
    };
    Cow::Owned(format!("{languages:?}; {pack}"))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let first = || load(&[1, 2, 3], "word", &[]).0.pack;
    let both = EngineMode::Both;
    match scenario {
        Scenario::FirstRun => {
            let (cold, fake) = load(&[1, 2, 3], "word", &[]);
            Ok(said(&cold, &fake))
        }
        Scenario::SameRules => {
            let (warm, fake) = load(&[1, 2, 3], "word", &[first()]);
            Ok(said(&warm, &fake))
        }
        Scenario::ChangedRule => {
            let (again, fake) = load(&[1, 2, 9], "word", &[first()]);
            Ok(said(&again, &fake))
        }
        Scenario::BrokenPack => {
            let broken = StoredPack {
                bytes: Bytes(b"no".to_vec()),
                ..first()
            };
            let (again, fake) = load(&[1, 2, 3], "word", &[broken]);
            Ok(said(&again, &fake))
        }
        Scenario::EmptyRuleSet => {
            let (engine, digest, kinds) = (use_case(), blake3(), kinds("words", "word"));
            let cold = loaded(
                &engine,
                &digest,
                LoadRequest {
                    plans: &[],
                    digests: &[],
                    kinds: &kinds,
                    stored: &[],
                },
            )
            .pack;
            let stored = [cold.clone()];
            let warm = loaded(
                &engine,
                &digest,
                LoadRequest {
                    plans: &[],
                    digests: &[],
                    kinds: &kinds,
                    stored: &stored,
                },
            );
            let same = if warm.pack == cold {
                "the pack kept as it was"
            } else {
                "another pack"
            };
            Ok(Cow::Owned(format!(
                "{:?}; accepted {:?}; {same}",
                warm.from, warm.accepted
            )))
        }
        Scenario::OldEngine => {
            let (pack, rules) = (first(), three_rules());
            let request = HarnessRequest {
                plans: &rules.iter().collect::<Vec<&Plan>>(),
                digests: &digests(&[1, 2, 3]),
                stored: std::slice::from_ref(&pack),
            };
            let load = harnessed_in(EngineMode::Old, None, request)
                .map_err(|error| Cow::Owned(error.to_string()))?;
            Ok(kept(&load, &pack))
        }
        Scenario::BothNothingAccepted | Scenario::NamedLanguage | Scenario::TwoLanguages => {
            let fake = EngineFake::default();
            let (first, second) = (kinds("words", "word"), kinds("signs", "sign"));
            let (linked, named, lowered) = match scenario {
                Scenario::NamedLanguage => (
                    vec![first, second.clone()],
                    Some(&second.language),
                    second.clone(),
                ),
                Scenario::TwoLanguages => (vec![first.clone(), second.clone()], None, first),
                _ => (vec![first.clone()], None, first),
            };
            let scenes = ["unbound correlated sub-plan"];
            let load = harness(both, Some(&fake), &scenes, linked, named)?;
            let refused = plans(&scenes);
            let request = LoadRequest {
                plans: &refused.iter().collect::<Vec<&Plan>>(),
                digests: &digests(&[1]),
                kinds: &lowered,
                stored: &[],
            };
            let expected = loaded(&EngineFake::default(), &blake3(), request).pack;
            Ok(kept(&load, &expected))
        }
        Scenario::NewEngine
        | Scenario::BothAccepted
        | Scenario::NoEngine
        | Scenario::NamedNotLinked => {
            let fake = EngineFake::default();
            let (words, signs) = (kinds("words", "word"), kinds("signs", "sign"));
            let packs = match scenario {
                Scenario::NewEngine => harness(
                    EngineMode::New,
                    Some(&fake),
                    &["one tree rule"],
                    vec![words],
                    None,
                ),
                Scenario::BothAccepted => {
                    harness(both, Some(&fake), &["one tree rule"], vec![words], None)
                }
                Scenario::NoEngine => harness(both, None, &[], vec![words], None),
                _ => {
                    let tongues = LanguageName("tongues".into());
                    harness(both, Some(&fake), &[], vec![words, signs], Some(&tongues))
                }
            }?;
            Ok(kept(&packs, &StoredPack::default()))
        }
    }
}

#[test]
fn succeeds() {
    let key = |seed, kind| pack_key(&blake3(), &digests(&[seed]), &kinds("words", kind));
    assert!(
        key(1, "word") != key(2, "word"),
        "a changed rule is another key"
    );
    assert!(
        key(1, "word") != key(1, "token"),
        "other kinds are another key"
    );
    let request = HarnessRequest {
        plans: &[],
        digests: &[],
        stored: &[],
    };
    let said = |answer: Result<Vec<HarnessLoad>, OperatingError>| {
        answer.map_or_else(|error| error.to_string(), |loads| loads.len().to_string())
    };
    let here = said(harnessed(None, request));
    assert_eq!(
        here,
        said(harnessed_in(engine_mode(), None, request)),
        "the mode of the process"
    );
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
