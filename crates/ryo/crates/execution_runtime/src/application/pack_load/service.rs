//! The `pack_load` service: the rule set of a run, loaded once before the files. The store's pack of the same key is read back,
//! and nothing is lowered; otherwise the selected rules are lowered and their pack is written for the store. On the build branch
//! it is where the `RYO_ENGINE` harness starts (scope-engine2 2.9, `old|new|both`): what the new engine's `load` accepted is
//! reported by count, and a run the new engine cannot finish is stopped. The comparison of the two engines' rows is the file
//! scan's; the use case that runs the files composes the two.

use super::constants::{NO_ENGINE_COMPOSED, NO_ONE_LANGUAGE};
use super::types::{Harness, HarnessLoad, HarnessRequest, LoadRequest, LoadedFrom, Refusals};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestBytes;
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::lowering::types::{SelectedRule, SelectedRules};
use ryo_engine::domain::packs::constants::ENGINE_VERSION;
use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
use ryo_engine::domain::programs::types::Programs;
use ryo_engine::inbound::use_cases::load::errors::LoadError;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::spike::{
    EngineMode, engine_loaded, engine_mode, engine_refusal, engine_unpacked,
};
use ryo_relations::domain::relations::types::{Bytes, Count};
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, LanguageName};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::{PackKey, StoredPack};

/// What a rule set's pack is kept under: what the rules' digests come to, in rule order, then what the kind vocabulary and the
/// engine's version come to. A changed rule, another language's kinds or another engine is another key.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::pack_key;
/// use ryo_values::domain::descriptions::types::{KindVocabulary, KindWord, LanguageName};
///
/// let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
/// let kinds = |word: &str| KindVocabulary { language: LanguageName::default(), kinds: vec![KindWord(word.into())] };
/// assert_ne!(pack_key(&digest, &[], &kinds("call")), pack_key(&digest, &[], &kinds("name")));
/// ```
#[must_use]
pub fn pack_key(digest: &dyn DigestPort, rules: &[Digest], kinds: &KindVocabulary) -> PackKey {
    // Each text after its length, so no two vocabularies write the same bytes.
    let mut written = Vec::new();
    let texts = std::iter::once(&*kinds.language.0)
        .chain(kinds.kinds.iter().map(|kind| &*kind.0))
        .chain(std::iter::once(ENGINE_VERSION));
    for each in texts {
        written.extend_from_slice(&u64::try_from(each.len()).unwrap_or(u64::MAX).to_le_bytes());
        written.extend_from_slice(each.as_bytes());
    }
    let (rules, vocabulary) = (digest.combine(rules), digest.digest(DigestBytes(&written)));
    let mut key = PackKey::default();
    let (first, second) = key.0.split_at_mut(rules.0.len());
    first.copy_from_slice(&rules.0);
    second.copy_from_slice(&vocabulary.0);
    key
}

/// The rule set of a run: the store's pack of the same key read back, with nothing lowered; otherwise each selected rule lowered
/// alone, the refused ones counted by why, then the accepted ones together, and their pack written for the store. A stored pack
/// the engine refuses (another engine version, bytes that do not decode) is lowered again.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::loaded;
/// use ryo_execution_runtime::application::pack_load::types::{LoadRequest, LoadedFrom};
/// use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
/// let kinds = KindVocabulary { language: LanguageName::default(), kinds: Vec::new() };
/// let cold = loaded(&engine, &digest, LoadRequest { plans: &[], digests: &[], kinds: &kinds, stored: &[] });
/// let stored = [cold.pack];
/// let warm = loaded(&engine, &digest, LoadRequest { plans: &[], digests: &[], kinds: &kinds, stored: &stored });
/// assert_eq!(warm.from, LoadedFrom::Pack);
/// ```
#[must_use]
pub fn loaded(
    engine: &dyn EnginePort,
    digest: &dyn DigestPort,
    request: LoadRequest<'_>,
) -> HarnessLoad {
    let key = pack_key(digest, request.digests, request.kinds);
    let written_for = PackDigest(key.0);
    if let Some(stored) = request.stored.iter().find(|pack| pack.key == key)
        && let Ok(programs) = engine.load(
            LoadSource::Pack(PackBytes(&stored.bytes.0), written_for),
            request.kinds,
        )
    {
        return HarnessLoad {
            accepted: engine.accepted(&programs),
            programs,
            from: LoadedFrom::Pack,
            pack: stored.clone(),
        };
    }
    let mut refusals = Refusals::default();
    let mut accepted: Vec<SelectedRule<'_>> = Vec::new();
    for (place, plan) in request.plans.iter().enumerate() {
        let rule = SelectedRule {
            rule: SelectedRuleOrdinal(u32::try_from(place).unwrap_or(u32::MAX)),
            plan,
        };
        match engine.load(LoadSource::Plans(SelectedRules(&[rule])), request.kinds) {
            Ok(_) => accepted.push(rule),
            Err(LoadError::Budget { .. }) => {
                refusals.over_budget = Count(refusals.over_budget.0.saturating_add(1));
            }
            Err(_) => refusals.unlowerable = Count(refusals.unlowerable.0.saturating_add(1)),
        }
    }
    // Rules lowered alone and refused together are refused: the rule set runs none of them.
    let programs = match engine.load(LoadSource::Plans(SelectedRules(&accepted)), request.kinds) {
        Ok(programs) => programs,
        Err(LoadError::Budget { .. }) => {
            refusals.over_budget = Count(refusals.over_budget.0.saturating_add(accepted.len()));
            Programs::default()
        }
        Err(_) => {
            refusals.unlowerable = Count(refusals.unlowerable.0.saturating_add(accepted.len()));
            Programs::default()
        }
    };
    let pack = StoredPack {
        key,
        bytes: Bytes(engine.encoded(&programs, written_for).0),
    };
    HarnessLoad {
        accepted: engine.accepted(&programs),
        programs,
        from: LoadedFrom::Lowering(refusals),
        pack,
    }
}

/// The kind vocabulary the rules are lowered against: the named language's, or the only linked one's.
fn lowered_against<'a>(
    vocabularies: &'a KindVocabularies,
    named: Option<&LanguageName>,
) -> Option<&'a KindVocabulary> {
    match (named, vocabularies.0.as_slice()) {
        (Some(name), linked) => linked.iter().find(|each| each.language == *name),
        (None, [only]) => Some(only),
        (None, _) => None,
    }
}

/// The harness of a request's file phase in `mode`, before any file. The old engine: nothing is loaded, and the store's packs
/// stay as they were. `new` and `both`: the rule set loaded against the linked language's kinds and reported by count. Answers
/// the load: its pack is what the store keeps, and its accepted rules are what `both` compares after the files.
///
/// # Errors
///
/// `EngineHarness` when the edge composed no engine, when no one language's kinds can be chosen, and when the new engine would
/// have to run files no linked language walks yet.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::harnessed_in;
/// use ryo_execution_runtime::application::pack_load::types::HarnessRequest;
/// use ryo_operators::spike::EngineMode;
///
/// let request = HarnessRequest { plans: &[], digests: &[], stored: &[] };
/// assert!(harnessed_in(EngineMode::Old, None, request).is_ok_and(|load| load.is_none()));
/// ```
pub fn harnessed_in(
    mode: EngineMode,
    harness: Option<Harness<'_>>,
    request: HarnessRequest<'_>,
) -> Result<Option<HarnessLoad>, OperatingError> {
    if mode == EngineMode::Old {
        return Ok(None);
    }
    let stopped = |reason| OperatingError::EngineHarness { reason };
    let harness = harness.ok_or_else(|| stopped(NO_ENGINE_COMPOSED.into()))?;
    let kinds = lowered_against(harness.vocabularies, harness.named)
        .ok_or_else(|| stopped(NO_ONE_LANGUAGE.into()))?;
    let load = loaded(
        harness.engine,
        harness.digest,
        LoadRequest {
            plans: request.plans,
            digests: request.digests,
            kinds,
            stored: request.stored,
        },
    );
    let (accepted, selected) = (load.accepted.len(), request.plans.len());
    match load.from {
        LoadedFrom::Lowering(refused) => engine_loaded(
            mode,
            accepted,
            selected,
            refused.unlowerable.0,
            refused.over_budget.0,
        ),
        LoadedFrom::Pack => engine_unpacked(mode, accepted, selected),
    }
    if mode == EngineMode::New || accepted > 0 {
        return Err(stopped(engine_refusal(mode, accepted)));
    }
    Ok(Some(load))
}

/// The harness of a request's file phase, in the mode `RYO_ENGINE` selects: see [`harnessed_in`].
///
/// # Errors
///
/// As [`harnessed_in`].
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::harnessed;
/// use ryo_execution_runtime::application::pack_load::types::{Harness, HarnessRequest};
/// use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, LanguageName};
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
/// let only = KindVocabulary { language: LanguageName::default(), kinds: Vec::new() };
/// let vocabularies = KindVocabularies(vec![only]);
/// let harness = Harness { engine: &engine, digest: &digest, vocabularies: &vocabularies, named: None };
/// let _load = harnessed(Some(harness), HarnessRequest { plans: &[], digests: &[], stored: &[] });
/// ```
pub fn harnessed(
    harness: Option<Harness<'_>>,
    request: HarnessRequest<'_>,
) -> Result<Option<HarnessLoad>, OperatingError> {
    harnessed_in(engine_mode(), harness, request)
}
