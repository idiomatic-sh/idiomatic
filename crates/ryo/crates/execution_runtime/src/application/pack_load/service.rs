//! The `pack_load` service: the rule set of a run, loaded once before the files. A pack is kept under what the run was given
//! (its rules as they were written, before any compile), the language's kinds and the engine's version: the store's pack of the
//! same key is read back, which needs no plan, so a run that finds it compiles and lowers nothing; otherwise the selected rules,
//! compiled, are lowered and their pack is written for the store. On the build branch it is where the `RYO_ENGINE` harness
//! starts (scope-engine2 2.9, `old|new|both`): what the new engine's `load` accepted is reported by count. The use case that runs
//! the files composes it with the file scan.

use super::constants::{NO_ENGINE_COMPOSED, NO_LANGUAGE, STATISTICS_MATCHED};
use super::selecting::service::{Lowered, lowered};
use super::types::{
    EngineVersion, Harness, HarnessLoad, HarnessRequest, KeptRequest, LoadRequest, LoadedFrom,
    ReadBackRequest,
};
use crate::outbound::digest::interface::DigestPort;
use crate::outbound::digest::types::DigestBytes;
use crate::outbound::engine::interface::EnginePort;
use ryo_engine::domain::packs::constants::ENGINE_VERSION;
use ryo_engine::domain::packs::types::{PackBytes, PackDigest};
use ryo_engine::domain::statistics::types::MatchedStatistics;
use ryo_engine::inbound::use_cases::load::types::LoadSource;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::spike::{EngineMode, engine_loaded, engine_said, engine_unpacked};
use ryo_relations::domain::relations::types::{Bytes, Count};
use ryo_values::domain::descriptions::types::{KindVocabularies, KindVocabulary, LanguageName};
use ryo_values::domain::stored_facts::types::Digest;
use ryo_values::domain::stores::types::{PackKey, StoredPack};

/// What a rule set's pack is kept under: `given`, what the run was given as one digest (the rules as they were written, their
/// fragments, what the host declared and which were selected: the caller's, made before any compile), then what the kind
/// vocabulary and the engine's version come to. A changed rule or fragment, another selection, another language's kinds or
/// another engine is another key.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::pack_key;
/// use ryo_values::domain::descriptions::types::{KindVocabulary, KindWord, LanguageName};
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
/// let kinds = |word: &str| KindVocabulary { language: LanguageName::default(), kinds: vec![KindWord(word.into())], fields: Vec::new(), fixed: Vec::new() };
/// assert_ne!(pack_key(&digest, Digest([0; 16]), &kinds("call")), pack_key(&digest, Digest([0; 16]), &kinds("name")));
/// assert_ne!(pack_key(&digest, Digest([0; 16]), &kinds("call")), pack_key(&digest, Digest([1; 16]), &kinds("call")));
/// ```
#[must_use]
pub fn pack_key(digest: &dyn DigestPort, given: Digest, kinds: &KindVocabulary) -> PackKey {
    keyed(digest, given, kinds, EngineVersion(ENGINE_VERSION))
}

/// The key of a pack written by the engine of `version`.
pub(super) fn keyed(
    digest: &dyn DigestPort,
    given: Digest,
    kinds: &KindVocabulary,
    version: EngineVersion<'_>,
) -> PackKey {
    // Each text after its length, and the end of each list (the kinds, the fields, the fixed names) as a length no text has:
    // no two vocabularies write the same bytes, and a name moved from one list to another is another key.
    let mut written = Vec::new();
    let end = std::iter::once(None);
    let texts = std::iter::once(Some(&*kinds.language.0))
        .chain(kinds.kinds.iter().map(|kind| Some(&*kind.0)))
        .chain(end.clone())
        .chain(kinds.fields.iter().map(|field| Some(&*field.0)))
        .chain(end.clone())
        .chain(kinds.fixed.iter().map(|name| Some(&*name.0)))
        .chain(end)
        .chain(std::iter::once(Some(version.0)));
    for each in texts {
        let length = each.map_or(u64::MAX, |text| {
            u64::try_from(text.len()).unwrap_or(u64::MAX)
        });
        written.extend_from_slice(&length.to_le_bytes());
        written.extend_from_slice(each.map_or(&[][..], str::as_bytes));
    }
    let vocabulary = digest.digest(DigestBytes(&written));
    let mut key = PackKey::default();
    let (first, second) = key.0.split_at_mut(given.0.len());
    first.copy_from_slice(&given.0);
    second.copy_from_slice(&vocabulary.0);
    key
}

/// The rule set of a run as the store kept it: its pack under the key of what the run was given, read back by the engine. No
/// plan is read: a run that finds it has compiled nothing. None when the store holds no pack under the key, when the engine
/// refuses the one it holds (another engine version, bytes that do not decode), and when the pack does not say what each rule
/// its programs run emits.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::kept;
/// use ryo_execution_runtime::application::pack_load::types::KeptRequest;
/// use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
/// let kinds = KindVocabulary { language: LanguageName::default(), kinds: Vec::new(), fields: Vec::new(), fixed: Vec::new() };
/// let request = KeptRequest { given: Digest([0; 16]), kinds: &kinds, stored: &[] };
/// assert!(kept(&engine, &digest, request).is_none());
/// ```
#[must_use]
pub fn kept(
    engine: &dyn EnginePort,
    digest: &dyn DigestPort,
    request: KeptRequest<'_>,
) -> Option<HarnessLoad> {
    let key = pack_key(digest, request.given, request.kinds);
    let stored = request.stored.iter().find(|pack| pack.key == key)?;
    let read = LoadSource::Pack(PackBytes(&stored.bytes.0), PackDigest(key.0));
    let Ok(loaded) = engine.load(read, request.kinds) else {
        return None;
    };
    let (programs, matched) = (loaded.programs, loaded.matched);
    let accepted = engine.accepted(&programs);
    // The host reads a rule's rows by what the pack says the rule emits: a pack that does not say it of each rule its programs
    // run is not this rule set's.
    if stored.emits.len() != accepted.len() {
        return None;
    }
    Some(HarnessLoad {
        language: request.kinds.language.clone(),
        accepted,
        programs,
        from: LoadedFrom::Pack,
        matched,
        pack: stored.clone(),
    })
}

/// The rule set of a run: the store's pack of the same key read back, with nothing lowered (`kept`); otherwise every selected
/// rule lowered in one load, each with what it binds its parameters to and its steps' digests, the refused ones counted by why, and the pack of the
/// accepted ones written for the store with what a row of each of them emits. A stored pack the engine refuses (another engine
/// version, bytes that do not decode) is lowered again.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::loaded;
/// use ryo_execution_runtime::application::pack_load::types::{Declared, LoadRequest, LoadedFrom};
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_values::domain::descriptions::types::{KindVocabulary, LanguageName};
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let engine = ryo_execution_runtime::outbound::engine::factory::use_case();
/// let digest = ryo_execution_runtime::outbound::digest::factory::blake3();
/// let kinds = KindVocabulary { language: LanguageName::default(), kinds: Vec::new(), fields: Vec::new(), fixed: Vec::new() };
/// let given = Digest([0; 16]);
/// let cold = loaded(&engine, &digest, LoadRequest { plans: &[], bindings: &[], statistics: ByteSlice(&[]), given, kinds: &kinds, stored: &[], declared: Declared::default() });
/// let stored = [cold.pack];
/// let warm = loaded(&engine, &digest, LoadRequest { plans: &[], bindings: &[], statistics: ByteSlice(&[]), given, kinds: &kinds, stored: &stored, declared: Declared::default() });
/// assert_eq!(warm.from, LoadedFrom::Pack);
/// ```
#[must_use]
pub fn loaded(
    engine: &dyn EnginePort,
    digest: &dyn DigestPort,
    request: LoadRequest<'_>,
) -> HarnessLoad {
    let stored = KeptRequest {
        given: request.given,
        kinds: request.kinds,
        stored: request.stored,
    };
    let key = pack_key(digest, request.given, request.kinds);
    if let Some(load) = kept(engine, digest, stored) {
        // Spike (ryo-fmx3.1): a hit checked against a fresh load of the same rules, when asked: the code a hit runs is
        // the code a miss writes, byte for byte.
        if std::env::var_os("RYO_L2_PACK_CHECK").is_some() {
            let fresh = lowered(engine, digest, &request).programs;
            let again = engine.encoded(&fresh, PackDigest(key.0)).0;
            let read = engine.encoded(&load.programs, PackDigest(key.0)).0;
            eprintln!(
                "ryo l2: pack check: stored {} bytes, fresh {} bytes, hit re-encoded {} bytes; fresh equal {}, hit equal {}",
                load.pack.bytes.0.len(),
                again.len(),
                read.len(),
                again == load.pack.bytes.0,
                read == load.pack.bytes.0
            );
        }
        return load;
    }
    let Lowered {
        programs,
        refusals,
        matched,
    } = lowered(engine, digest, &request);
    let accepted = engine.accepted(&programs);
    let pack = StoredPack {
        key,
        bytes: Bytes(engine.encoded(&programs, PackDigest(key.0)).0),
        emits: accepted
            .iter()
            .filter_map(|rule| {
                request
                    .plans
                    .get(usize::try_from(rule.0).unwrap_or(usize::MAX))
            })
            .map(|plan| plan.emit.clone())
            .collect(),
    };
    HarnessLoad {
        language: request.kinds.language.clone(),
        accepted,
        programs,
        from: LoadedFrom::Lowering(refusals),
        matched,
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

/// The languages a run in `mode` loads a rule set for, each by its kinds: none under the old engine alone.
fn languages<'h>(
    mode: EngineMode,
    harness: Option<&Harness<'h>>,
) -> Result<Vec<&'h KindVocabulary>, OperatingError> {
    if mode == EngineMode::Old {
        return Ok(Vec::new());
    }
    let stopped = |reason| OperatingError::EngineHarness { reason };
    let harness = harness.ok_or_else(|| stopped(NO_ENGINE_COMPOSED.into()))?;
    let languages = lowered_against(harness.vocabularies, harness.named);
    if languages.is_empty() {
        return Err(stopped(NO_LANGUAGE.into()));
    }
    Ok(languages)
}

/// Says a load by count, as the harness reports it: what was lowered and refused, or that the store's pack was read back.
fn said(mode: EngineMode, load: &HarnessLoad, selected: Count) {
    let ran = load.accepted.len();
    match load.from {
        LoadedFrom::Lowering(refused) => engine_loaded(
            mode,
            ran,
            selected.0,
            refused.unlowerable.0,
            refused.past_width.0,
        ),
        LoadedFrom::Pack => engine_unpacked(mode, ran, selected.0),
    }
    // A load that read statistics says how many of its units they name: a load that read none says nothing more.
    match load.matched {
        MatchedStatistics::Units(units) => {
            let [before, after] = STATISTICS_MATCHED;
            engine_said(mode, &[before, &units.0.to_string(), after].concat());
        }
        MatchedStatistics::Absent => {}
    }
}

/// The rule sets of a request as the store kept them, before anything is compiled, in `mode`. The old engine: none, and the
/// store's packs are not read. `new` and `both`: one rule set for each linked language (or for the one the request names), each
/// read back from the store's pack of what the run was given and reported by count; nothing when the store lacks the pack of one
/// of them, and then nothing is reported: the run compiles its rules and asks `harnessed`.
///
/// # Errors
///
/// `EngineHarness` when the edge composed no engine, and when there is no language to lower against.
///
/// # Examples
///
/// ```
/// use ryo_execution_runtime::application::pack_load::service::read_back;
/// use ryo_execution_runtime::application::pack_load::types::ReadBackRequest;
/// use ryo_operators::spike::EngineMode;
/// use ryo_relations::domain::relations::types::Count;
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let request = ReadBackRequest { given: Digest([0; 16]), selected: Count(0), stored: &[] };
/// assert!(read_back(EngineMode::Old, None, request).is_ok_and(|loads| loads.is_some_and(|loads| loads.is_empty())));
/// ```
pub fn read_back(
    mode: EngineMode,
    harness: Option<Harness<'_>>,
    request: ReadBackRequest<'_>,
) -> Result<Option<Vec<HarnessLoad>>, OperatingError> {
    let languages = languages(mode, harness.as_ref())?;
    let Some(harness) = harness.filter(|_| !languages.is_empty()) else {
        return Ok(Some(Vec::new()));
    };
    let mut loads = Vec::new();
    for kinds in languages {
        let read = KeptRequest {
            given: request.given,
            kinds,
            stored: request.stored,
        };
        let Some(load) = kept(harness.engine, harness.digest, read) else {
            return Ok(None);
        };
        loads.push(load);
    }
    for load in &loads {
        said(mode, load, request.selected);
    }
    Ok(Some(loads))
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
/// use ryo_execution_runtime::application::pack_load::types::{Declared, HarnessRequest};
/// use ryo_language::domain::symbols::types::ByteSlice;
/// use ryo_operators::spike::EngineMode;
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let request = HarnessRequest { plans: &[], bindings: &[], statistics: ByteSlice(&[]), given: Digest([0; 16]), stored: &[], declared: Declared::default() };
/// assert!(harnessed(EngineMode::Old, None, request).is_ok_and(|loads| loads.is_empty()));
/// ```
pub fn harnessed(
    mode: EngineMode,
    harness: Option<Harness<'_>>,
    request: HarnessRequest<'_>,
) -> Result<Vec<HarnessLoad>, OperatingError> {
    let languages = languages(mode, harness.as_ref())?;
    let Some(harness) = harness.filter(|_| !languages.is_empty()) else {
        return Ok(Vec::new());
    };
    let mut loads = Vec::new();
    for kinds in languages {
        let load = loaded(
            harness.engine,
            harness.digest,
            LoadRequest {
                plans: request.plans,
                bindings: request.bindings,
                statistics: request.statistics,
                given: request.given,
                kinds,
                stored: request.stored,
                declared: request.declared,
            },
        );
        said(mode, &load, Count(request.plans.len()));
        loads.push(load);
    }
    Ok(loads)
}
