//! The cases of reading a rule set back from the store before anything is compiled, over the engine port's fake: the pack of
//! what the run was given is read back with what its rule emits and nothing is lowered; nothing is read back when the store
//! holds no pack, when what the run was given, the language's kinds or the engine's version differ, or when the pack does not say
//! what its rules emit; and the harness answers a run's rule sets from the store only when the store holds every one.

use super::super::{given, kinds, load, three_rules};
use crate::application::pack_load::service::{kept, keyed, loaded, pack_key, read_back};
use crate::application::pack_load::types::{
    EngineVersion, Harness, KeptRequest, LoadRequest, ReadBackRequest,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::digest::factory::blake3;
use crate::outbound::engine::fixtures::fake::EngineFake;
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_operators::spike::EngineMode;
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::descriptions::types::{
    FieldWord, FixedName, KindVocabularies, KindVocabulary,
};
use ryo_values::domain::query_plans::types::Plan;
use ryo_values::domain::stores::types::StoredPack;
use std::borrow::Cow;
use std::sync::atomic::Ordering;

/// What a row does.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    /// The first run's pack stored, read back with no plan in hand.
    KeptWithoutPlans,
    /// Nothing stored, and no plan in hand.
    NothingKept,
    /// Something else given than when the pack was stored, and no plan in hand.
    KeptOfOtherRules,
    /// The first run's pack stored, read for a language of another kind, with no plan in hand.
    KeptOfOtherKinds,
    /// The first run's pack stored under the key another engine version writes, and no plan in hand.
    KeptByAnotherVersion,
    /// The first run's pack stored without what its rule emits, and no plan in hand.
    KeptWithoutEmits,
    /// `new`, two languages linked, the store holding the pack of each: asked before anything is compiled.
    StoredBoth,
    /// `new`, two languages linked, the store holding the pack of the first alone: asked before anything is compiled.
    StoredOne,
    /// The old engine, asked of the store before anything is compiled.
    StoredOld,
    /// The key of one language under vocabularies that differ in one field name, in one fixed name, and in which list holds
    /// one name; and under the same names twice.
    KeyedByEveryName,
}

type Input = Scenario;
/// What was read back, and what the fake was asked, said in one line.
type Value = Cow<'static, str>;
/// Why the harness stopped the run.
type Failure = Cow<'static, str>;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_stored_pack_is_read_back_with_no_plan_in_hand"),
        word: Word::Succeeds,
        input: || Scenario::KeptWithoutPlans,
        expect: Expect::Value(Cow::Borrowed(
            "Pack; accepted [0]; rule 0 subject 0 key [0] headings [\"node\"]; lowerings 0; packs read 1",
        )),
    },
    Case {
        name: CaseName("a_store_without_the_pack_reads_nothing_back"),
        word: Word::Succeeds,
        input: || Scenario::NothingKept,
        expect: Expect::Value(Cow::Borrowed(
            "nothing read back; lowerings 0; packs read 0",
        )),
    },
    Case {
        name: CaseName("a_pack_of_something_else_given_is_not_read_back"),
        word: Word::Succeeds,
        input: || Scenario::KeptOfOtherRules,
        expect: Expect::Value(Cow::Borrowed(
            "nothing read back; lowerings 0; packs read 0",
        )),
    },
    Case {
        name: CaseName("a_pack_of_other_kinds_is_not_read_back"),
        word: Word::Succeeds,
        input: || Scenario::KeptOfOtherKinds,
        expect: Expect::Value(Cow::Borrowed(
            "nothing read back; lowerings 0; packs read 0",
        )),
    },
    Case {
        name: CaseName("a_pack_of_another_engine_version_is_not_read_back"),
        word: Word::Succeeds,
        input: || Scenario::KeptByAnotherVersion,
        expect: Expect::Value(Cow::Borrowed(
            "nothing read back; lowerings 0; packs read 0",
        )),
    },
    Case {
        name: CaseName("a_pack_that_does_not_say_what_its_rules_emit_is_not_read_back"),
        word: Word::Succeeds,
        input: || Scenario::KeptWithoutEmits,
        expect: Expect::Value(Cow::Borrowed(
            "nothing read back; lowerings 0; packs read 1",
        )),
    },
    Case {
        name: CaseName("the_store_answers_every_linked_languages_rule_set_before_a_compile"),
        word: Word::Succeeds,
        input: || Scenario::StoredBoth,
        expect: Expect::Value(Cow::Borrowed(
            "[(\"words\", 1), (\"signs\", 1)]; lowerings 0; packs read 2",
        )),
    },
    Case {
        name: CaseName("a_store_that_lacks_one_languages_pack_answers_no_rule_set"),
        word: Word::Succeeds,
        input: || Scenario::StoredOne,
        expect: Expect::Value(Cow::Borrowed("no rule set; lowerings 0; packs read 1")),
    },
    Case {
        name: CaseName("the_old_engine_asks_nothing_of_the_store"),
        word: Word::Succeeds,
        input: || Scenario::StoredOld,
        expect: Expect::Value(Cow::Borrowed("[]; lowerings 0; packs read 0")),
    },
    Case {
        name: CaseName("a_pack_is_keyed_by_the_field_names_and_the_fixed_names_too"),
        word: Word::Succeeds,
        input: || Scenario::KeyedByEveryName,
        expect: Expect::Value(Cow::Borrowed(
            "a field name more: differ; a fixed name more: differ; the kind's name among the fields: differ; the same names: equal",
        )),
    },
];

/// Whether two vocabularies of one language key a pack alike, said of `other` against the language of one kind `word`.
fn keyed_as(other: &KindVocabulary) -> &'static str {
    let key = |names: &KindVocabulary| keyed(&blake3(), given(1), names, EngineVersion("1"));
    if key(other) == key(&kinds("words", "word")) {
        "equal"
    } else {
        "differ"
    }
}

/// The keys of vocabularies that differ from one language's in one name of one list.
fn keyed_by_every_name() -> Value {
    let plain = kinds("words", "word");
    let field = KindVocabulary {
        fields: vec![FieldWord("label".into())],
        ..plain.clone()
    };
    let fixed = KindVocabulary {
        fixed: vec![FixedName("next".into())],
        ..plain.clone()
    };
    let moved = KindVocabulary {
        kinds: Vec::new(),
        fields: vec![FieldWord("word".into())],
        ..plain.clone()
    };
    Cow::Owned(format!(
        "a field name more: {}; a fixed name more: {}; the kind's name among the fields: {}; the same names: {}",
        keyed_as(&field),
        keyed_as(&fixed),
        keyed_as(&moved),
        keyed_as(&plain),
    ))
}

/// What a new fake reads back of `stored`, given `seed` and `kind`, with no plan in hand: the rule set and what its pack says
/// each of its rules emits, or that nothing was read back; then what the fake was asked.
fn kept_of(seed: u8, kind: &str, stored: &[StoredPack]) -> Value {
    let (fake, kinds) = (EngineFake::default(), kinds("words", kind));
    let request = KeptRequest {
        given: given(seed),
        kinds: &kinds,
        stored,
    };
    let (lowered, unpacked) = (&fake.lowered, &fake.unpacked);
    let asked = |fake_said: String| {
        Cow::Owned(format!(
            "{fake_said}; lowerings {}; packs read {}",
            lowered.load(Ordering::Relaxed),
            unpacked.load(Ordering::Relaxed)
        ))
    };
    match kept(&fake, &blake3(), request) {
        Some(load) => {
            let rules: Vec<String> = load
                .accepted
                .iter()
                .zip(&load.pack.emits)
                .map(|(rule, emit)| {
                    let headings: Vec<&str> = emit.headings.iter().map(|each| &*each.0).collect();
                    format!(
                        "rule {} subject {} key {:?} headings {headings:?}",
                        rule.0,
                        emit.subject.0,
                        emit.key.iter().map(|each| each.0).collect::<Vec<u32>>()
                    )
                })
                .collect();
            let accepted: Vec<u32> = load.accepted.iter().map(|rule| rule.0).collect();
            asked(format!(
                "{:?}; accepted {accepted:?}; {}",
                load.from,
                rules.join(", ")
            ))
        }
        None => asked("nothing read back".to_owned()),
    }
}

/// What the store answers a `new` run of two linked languages before anything is compiled, when it holds the packs of
/// `holding` of them: each rule set's language, or that it answers none; then what the fake was asked.
fn asked_of_the_store(mode: EngineMode, holding: &[&str]) -> Result<Value, Failure> {
    let linked = [kinds("words", "word"), kinds("signs", "sign")];
    let rules = three_rules();
    let packs: Vec<StoredPack> = linked
        .iter()
        .filter(|each| holding.contains(&&*each.language.0))
        .map(|each| {
            let request = LoadRequest {
                plans: &rules.iter().collect::<Vec<&Plan>>(),
                bindings: &[],
                statistics: ByteSlice(&[]),
                given: given(1),
                kinds: each,
                stored: &[],
                declared: Default::default(),
            };
            loaded(&EngineFake::default(), &blake3(), request).pack
        })
        .collect();
    let (fake, digest, vocabularies) = (
        EngineFake::default(),
        blake3(),
        KindVocabularies(linked.to_vec()),
    );
    let harness = Harness {
        engine: &fake,
        digest: &digest,
        vocabularies: &vocabularies,
        named: None,
    };
    let request = ReadBackRequest {
        given: given(1),
        selected: Count(rules.len()),
        stored: &packs,
    };
    let answered =
        read_back(mode, Some(harness), request).map_err(|error| Cow::Owned(error.to_string()))?;
    let said = match &answered {
        Some(loads) => format!(
            "{:?}",
            loads
                .iter()
                .map(|load| (&*load.language.0, load.accepted.len()))
                .collect::<Vec<_>>()
        ),
        None => "no rule set".to_owned(),
    };
    Ok(Cow::Owned(format!(
        "{said}; lowerings {}; packs read {}",
        fake.lowered.load(Ordering::Relaxed),
        fake.unpacked.load(Ordering::Relaxed)
    )))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let first = || load(1, "word", &[]).0.pack;
    match scenario {
        Scenario::KeptWithoutPlans => Ok(kept_of(1, "word", &[first()])),
        Scenario::NothingKept => Ok(kept_of(1, "word", &[])),
        Scenario::KeptOfOtherRules => Ok(kept_of(9, "word", &[first()])),
        Scenario::KeptOfOtherKinds => Ok(kept_of(1, "token", &[first()])),
        Scenario::KeptByAnotherVersion => {
            // The pack as the engine of another version keeps it: under that version's key.
            let other = StoredPack {
                key: keyed(
                    &blake3(),
                    given(1),
                    &kinds("words", "word"),
                    EngineVersion("0.0.0-other"),
                ),
                ..first()
            };
            Ok(kept_of(1, "word", &[other]))
        }
        Scenario::KeptWithoutEmits => {
            let unsaid = StoredPack {
                emits: Vec::new(),
                ..first()
            };
            Ok(kept_of(1, "word", &[unsaid]))
        }
        Scenario::StoredBoth => asked_of_the_store(EngineMode::New, &["words", "signs"]),
        Scenario::StoredOne => asked_of_the_store(EngineMode::New, &["words"]),
        Scenario::StoredOld => asked_of_the_store(EngineMode::Old, &["words", "signs"]),
        Scenario::KeyedByEveryName => Ok(keyed_by_every_name()),
    }
}

#[test]
fn succeeds() {
    let key = |seed, kind| pack_key(&blake3(), given(seed), &kinds("words", kind));
    assert!(
        key(1, "word")
            != keyed(
                &blake3(),
                given(1),
                &kinds("words", "word"),
                EngineVersion("0.0.0-other")
            ),
        "another engine version is another key"
    );
    run(CASES, Word::Succeeds, subject);
}
