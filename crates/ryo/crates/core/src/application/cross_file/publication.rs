//! Normalize demanded phase roots at the existing table codec boundary before Symbols are dropped.
use crate::domain::crossing::types::{CrossPublicationTransfer, PackedObservedRoot};
use crate::domain::{
    crossing::types::ObservedRoot,
    finish::publication::escape_cross_publication,
    table_words::{
        service::{encode_table, pack},
        types::WordSlice,
    },
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Origin, Words, WorkerNumber};
use ryo_values::domain::{
    observations::types::{ObservationReason, ProducerReadiness},
    stored_facts::types::{ArenaBytes, KeptWords},
};

/// The coordinator reserved `worker`; this closure owns only the new arena, never inherited bytes.
pub(crate) fn pack_roots(
    roots: Vec<ObservedRoot>,
    symbols: &mut Symbols,
    inherited: &[ArenaBytes],
    worker: WorkerNumber,
) -> CrossPublicationTransfer {
    let mut arena = ArenaBytes::default();
    let mut packed = Vec::with_capacity(roots.len());
    for root in roots {
        let mut published = PackedObservedRoot {
            output: root.output,
            words: None,
            readiness: root.readiness,
            arenas: Vec::new(),
        };
        if published.readiness == ProducerReadiness::Complete {
            if let Some(mut table) = root.table {
                let before = arena.0.len();
                let mut places = Vec::new();
                match escape_cross_publication(
                    &mut table,
                    symbols,
                    inherited,
                    worker,
                    &mut arena,
                    &mut places,
                ) {
                    Ok(()) => {
                        for place in &places {
                            if let Origin::Derived(origin) = place.origin
                                && !published.arenas.contains(&origin)
                            {
                                published.arenas.push(origin);
                            }
                        }
                        let mut words = Words::default();
                        encode_table(&table, &places, &mut words);
                        published.words = Some(KeptWords(pack(WordSlice(&words.0))));
                    }
                    Err(error) => {
                        // Roll back only this unpublished root's appended payload.
                        arena.0.truncate(before);
                        published.readiness =
                            ProducerReadiness::Unknown(ObservationReason(error.to_string().into()));
                    }
                }
            } else {
                published.readiness = ProducerReadiness::Unknown(ObservationReason(
                    "complete observation lacks its actual table".into(),
                ));
            }
        }
        packed.push(published);
    }
    CrossPublicationTransfer {
        worker,
        arena,
        roots: packed,
    }
}
