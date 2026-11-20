//! What a file read came to, as the store keeps it.
use super::super::service::FileAnalysis;
use super::super::types::{FileVisit, Worker};
use crate::domain::keeping::service::rows_of;
use crate::domain::recording::service::{ascending, frontier_flat_in};
use crate::domain::recording::service::interface_bytes;
use crate::domain::recording::service::made_ran;
use crate::domain::recording::service::moved_words;
use crate::domain::recording::types::Flat;
use crate::domain::render::types::Answering;
use crate::domain::visit::types::Keeping;
use ryo_execution::inbound::use_cases::execution::interface::NamedLayout;
use ryo_execution::inbound::use_cases::execution::interface::ProgramPreparedExecutionPort as _;
use ryo_execution::inbound::use_cases::metadata::interface::RuleIndex;
use ryo_execution_runtime::outbound::digest::types::DigestBytes;
use ryo_execution_runtime::outbound::digest::types::DigestWords;
use ryo_relations::domain::relations::types::FileId;
use ryo_values::domain::files::types::FileNumber;
use ryo_values::domain::records::types::Handed;
use ryo_values::domain::records::types::Handing;
use ryo_values::domain::records::types::Ran;
use ryo_values::domain::records::types::RuleFrontier;
use ryo_values::domain::row_texts::types::PlacedRule;
use ryo_values::domain::stored_facts::types::ArenaBase;
use ryo_values::domain::stored_facts::types::Digest;

impl FileAnalysis {
    /// What a file read came to, as the store keeps it: its findings, and what it shows other files: what its language noted of it,
    /// the relations it is read in part and what it hands the cross-file steps. Nothing when the run keeps nothing, or the words cannot
    /// be moved to another run.
    pub(in crate::application::file_analysis) fn recorded(
        &self,
        worker: &Worker,
        visit: &FileVisit<'_>,
        number: FileId,
        base: ArenaBase,
    ) -> Option<Ran> {
        let (setting, tables) = (&visit.setting, &worker.tables);
        if matches!(setting.answering, Answering::Rows(_)) {
            return None;
        }
        // A run whose store keeps nothing makes its findings alone: no words moved for another run, no digest of what it shows.
        if setting.keeping == Keeping::Discards {
            return Some(Ran {
                rows: rows_of(&tables.rendered),
                number: FileNumber(number.0),
                interface: Digest([0; 16]),
                frontier: Vec::new(),
                noted: Digest([0; 16]),
                handed: Handing::Made(Handed::default()),
            });
        }
        let moved = moved_words(tables, number, base)?;
        let clean = crate::domain::recording::service::clean_moved(tables, number, base)?;
        let layout = setting.program.observation_layout().0;
        let (observations, slots) = match (tables.current_observations.as_deref(), layout) {
            (
                Some(map),
                Some(NamedLayout {
                    layout,
                    frontiers: slots,
                }),
            ) => {
                let Ok(slot_count) = u32::try_from(slots.len()) else {
                    return None;
                };
                let count =
                    ryo_values::domain::records::types::ObservationReadinessCount(slot_count);
                let Ok(checked) = ryo_values::domain::observations::service::validate_frontiers(
                    map, layout, count,
                ) else {
                    return None;
                };
                (Some(checked), slots)
            }
            (
                None,
                Some(NamedLayout {
                    frontiers: slots, ..
                }),
            ) if !slots.is_empty() => return None,
            (Some(_), None) => return None,
            _ => (None, &[][..]),
        };
        let mut flat = Flat::default();
        let in_order = ascending(&moved);
        // Spike (ryo-fmx3, host3): buckets out of order are searched through their order by bucket, not scanned once per rule;
        // and a rule's words digested in this worker before are not digested again.
        let lever = ryo_operators::spike::host_lever(ryo_operators::spike::HostLever::Record);
        let mut searched: Vec<&crate::domain::recording::types::BucketWords> = moved.iter().collect();
        // Spike (ryo-fmx3, host3): a clean bucket is read with its moved clean frame, so each rule's digest is the one its words
        // would make.
        searched.extend(clean.iter().map(|(_, words)| words));
        let in_order = in_order && clean.is_empty();
        let searched_in_order = if lever && !in_order {
            // A stable sort: a rule's buckets, made in input order, keep their order.
            searched.sort_by_key(|each| each.bucket.0);
            true
        } else {
            in_order
        };
        let searched = searched.as_slice();
        let frontier: Vec<Digest> = (0..setting.metadata.len())
            .map(|rule| {
                frontier_flat_in(
                    searched,
                    searched_in_order,
                    observations,
                    slots,
                    setting,
                    RuleIndex(rule),
                    &mut flat,
                );
                let digest = if flat.0.is_empty() {
                    Digest([0; 16])
                } else if lever {
                    crate::domain::recording::service::remembered_digest(rule, &flat, |words| {
                        self.digest.digest_words(DigestWords(words))
                    })
                } else {
                    self.digest.digest_words(DigestWords(&flat.0))
                };
                flat.0.clear();
                digest
            })
            .collect();
        let noted = self
            .digest
            .digest(DigestBytes(&interface_bytes(tables, number).0));
        let mut shown = vec![noted];
        shown.extend(frontier.iter().copied());
        let mut ran = made_ran(tables, number, self.digest.combine(&shown), moved, base);
        if let ryo_values::domain::records::types::Handing::Made(handed) = &mut ran.handed {
            handed.clean = tables
                .clean
                .iter()
                .zip(&clean)
                .map(|(bucket, (frame, _))| ryo_values::domain::records::types::HandedClean {
                    bucket: *bucket,
                    frame: std::sync::Arc::clone(frame),
                })
                .collect();
        }
        ran.noted = noted;
        ran.frontier = frontier
            .iter()
            .enumerate()
            .filter(|(_, digest)| **digest != Digest([0; 16]))
            .map(|(rule, digest)| RuleFrontier {
                rule: PlacedRule(u32::try_from(rule).unwrap_or(u32::MAX)),
                digest: *digest,
            })
            .collect();
        Some(ran)
    }
}
