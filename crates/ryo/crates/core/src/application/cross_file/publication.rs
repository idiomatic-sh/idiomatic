//! Normalize demanded phase roots at the existing table codec boundary before Symbols are dropped.
use crate::domain::finish::service::missing_publication_table;
use ryo_execution::outbound::publication::interface::PublicationPort;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Origin, Words, WorkerNumber};
use ryo_values::domain::records::types::{CrossPublicationTransfer, PackedObservedRoot};
use ryo_values::domain::{
    records::types::{ObservationReason, ProducerReadiness},
    stored_facts::types::{ArenaBytes, KeptWords},
};
use {
    crate::domain::crossing::types::ObservedRoot,
    ryo_values::domain::table_words::service::encode_table,
    ryo_values::domain::table_words::service::pack,
    ryo_values::domain::table_words::types::WordSlice,
};

/// The coordinator reserved `worker`; this closure owns only the new arena, never inherited bytes.
pub(crate) fn pack_roots(
    roots: Vec<ObservedRoot>,
    symbols: &mut Symbols,
    inherited: &[ArenaBytes],
    worker: WorkerNumber,
    publication: &dyn PublicationPort,
) -> CrossPublicationTransfer {
    let mut arena = ArenaBytes::default();
    let mut packed = Vec::with_capacity(roots.len());
    for root in roots {
        packed.push(pack_root(
            root,
            symbols,
            inherited,
            worker,
            &mut arena,
            publication,
        ));
    }
    CrossPublicationTransfer {
        worker,
        arena,
        roots: packed,
    }
}

/// A failed root rolls back only its new bytes; earlier complete roots remain available.
fn pack_root(
    root: ObservedRoot,
    symbols: &mut Symbols,
    inherited: &[ArenaBytes],
    worker: WorkerNumber,
    arena: &mut ArenaBytes,
    publication: &dyn PublicationPort,
) -> PackedObservedRoot {
    let mut published = PackedObservedRoot {
        output: root.output,
        words: ryo_values::domain::records::types::RootWords::Withheld,
        readiness: root.readiness,
        arenas: Vec::new(),
    };
    match &published.readiness {
        ProducerReadiness::Complete => {}
        ProducerReadiness::Unknown(_) if root.table.is_some() => {}
        _ => return published,
    }
    let Some(mut table) = root.table else {
        published.readiness = ProducerReadiness::Unknown(ObservationReason(
            missing_publication_table().to_string().into(),
        ));
        return published;
    };
    let before = arena.0.len();
    let mut places = Vec::new();
    if let Err(reason) =
        publication.publication(&mut table, symbols, inherited, worker, arena, &mut places)
    {
        arena.0.truncate(before);
        published.readiness = ProducerReadiness::Unknown(reason);
        return published;
    }
    for place in &places {
        if let Origin::Derived(origin) = place.origin
            && !published.arenas.contains(&origin)
        {
            published.arenas.push(origin);
        }
    }
    let mut words = Words::default();
    encode_table(&table, &places, &mut words);
    published.words = ryo_values::domain::records::types::RootWords::Published(KeptWords(pack(
        WordSlice(&words.0),
    )));
    published
}
