//! The `pack_load` service: the rule set of a run, loaded once before the files. The store's pack of the same key is read back,
//! and nothing is lowered; otherwise the selected rules are lowered and their pack is written for the store. On the build branch
//! it is where the `RYO_ENGINE` harness starts (scope-engine2 2.9, `old|new|both`): what the new engine's `load` accepted is
//! reported by count. The use case that runs the files composes it with the file scan.

use super::constants::{NO_ENGINE_COMPOSED, NO_LANGUAGE};
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
use ryo_operators::spike::{EngineMode, engine_loaded, engine_unpacked};
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
            language: request.kinds.language.clone(),
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
        language: request.kinds.language.clone(),
        accepted: engine.accepted(&programs),
        programs,
        from: LoadedFrom::Lowering(refusals),
        pack,
    }
}

/// The kind vocabularies the rules are lowered against, one rule set each: the named language's, or every linked one's.
fn lowered_against<'a>(
    vocabularies: &'a KindVocabularies,
    named: Option<&LanguageName>,
) -> Vec<&'a KindVocabulary> {
    vocabularies
        .0
        .iter()
        .filter(|each| named.is_none_or(|name| each.language == *name))
        .collect()
}

/// The harness of a request's file phase in `mode`, before any file. The old engine: nothing is loaded, and the store's packs
/// are not asked for. `new` and `both`: one rule set for each linked language (or for the one the request names), loaded
/// against that language's kinds and reported by count. Answers the loads: each one's pack is what the store keeps, and its
/// accepted rules are what the files are scanned with, and what `both` compares.
///
/// # Errors
///
/// `EngineHarness` when the edge composed no engine, and when there is no language to lower against.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::harnessed;
/// use ryo_execution_runtime::application::pack_load::types::HarnessRequest;
/// use ryo_operators::spike::EngineMode;
///
/// let request = HarnessRequest { plans: &[], digests: &[], stored: &[] };
/// assert!(harnessed(EngineMode::Old, None, request).is_ok_and(|loads| loads.is_empty()));
/// ```
pub fn harnessed(
    mode: EngineMode,
    harness: Option<Harness<'_>>,
    request: HarnessRequest<'_>,
) -> Result<Vec<HarnessLoad>, OperatingError> {
    if mode == EngineMode::Old {
        return Ok(Vec::new());
    }
    let stopped = |reason| OperatingError::EngineHarness { reason };
    let harness = harness.ok_or_else(|| stopped(NO_ENGINE_COMPOSED.into()))?;
    let languages = lowered_against(harness.vocabularies, harness.named);
    if languages.is_empty() {
        return Err(stopped(NO_LANGUAGE.into()));
    }
    let mut loads = Vec::new();
    for kinds in languages {
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
        let (ran, selected) = (load.accepted.len(), request.plans.len());
        match load.from {
            LoadedFrom::Lowering(refused) => engine_loaded(
                mode,
                ran,
                selected,
                refused.unlowerable.0,
                refused.over_budget.0,
            ),
            LoadedFrom::Pack => engine_unpacked(mode, ran, selected),
        }
        loads.push(load);
    }
    Ok(loads)
}
