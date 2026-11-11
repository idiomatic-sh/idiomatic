//! Portable publication keeps earlier roots when a later root refuses.
use super::service::pack_roots;
use crate::domain::crossing::types::ObservedRoot;
use ryo_execution::inbound::use_cases::publication::interface::{
    PublicationError, PublicationInput, PublicationReason, PublicationUseCase,
};
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell, WorkerNumber};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        records::types::{ObservationIndex, ObservationReason, ProducerReadiness},
        rows::types::{RowCount, Table},
    },
};
use std::sync::atomic::{AtomicUsize, Ordering};

struct RefusingPublication(AtomicUsize);
impl PublicationUseCase for RefusingPublication {
    fn publication(&self, input: PublicationInput<'_>) -> Result<(), PublicationError> {
        let call = self.0.fetch_add(1, Ordering::SeqCst);
        input.arena.0.push(if call == 0 { 8 } else { 9 });
        if call == 0 {
            Ok(())
        } else {
            Err(PublicationError::Publication {
                reason: PublicationReason::ReservedArena,
            })
        }
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    LaterRefusal,
    MissingTable,
    Skipped,
    Partial,
}
type Input = Scenario;
/// The arena bytes kept, the publication calls made, and each root's words and readiness.
type Value = &'static str;
type Failure = std::convert::Infallible;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("partial_publication_keeps_words_without_rewriting_unknown_readiness"),
        word: Word::Succeeds,
        input: || Scenario::Partial,
        expect: Expect::Value("arena [8], 1 call(s), published unknown"),
    },
    Case {
        name: CaseName("later_refusal_keeps_only_prior_arena_bytes"),
        word: Word::Succeeds,
        input: || Scenario::LaterRefusal,
        expect: Expect::Value("arena [8], 2 call(s), published complete, withheld unknown"),
    },
    Case {
        name: CaseName("complete_without_table_is_unknown"),
        word: Word::Succeeds,
        input: || Scenario::MissingTable,
        expect: Expect::Value("arena [], 0 call(s), withheld unknown"),
    },
    Case {
        name: CaseName("skipped_root_is_not_normalized"),
        word: Word::Succeeds,
        input: || Scenario::Skipped,
        expect: Expect::Value("arena [], 0 call(s), withheld skipped"),
    },
];
fn numeric() -> Table {
    Table {
        kinds: vec![CellKind::Number],
        cells: vec![Cell::Number(Integer(1))],
        count: RowCount(1),
    }
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let root = |output, table, readiness| ObservedRoot {
        output: ObservationIndex(output),
        table,
        readiness,
    };
    let roots = match scenario {
        Scenario::LaterRefusal => vec![
            root(0, Some(numeric()), ProducerReadiness::Complete),
            root(1, Some(numeric()), ProducerReadiness::Complete),
        ],
        Scenario::MissingTable => vec![root(0, None, ProducerReadiness::Complete)],
        Scenario::Skipped => vec![root(0, Some(numeric()), ProducerReadiness::Skipped)],
        Scenario::Partial => vec![root(
            0,
            Some(numeric()),
            ProducerReadiness::Unknown(ObservationReason("partial physical scope".into())),
        )],
    };
    let port = RefusingPublication(AtomicUsize::new(0));
    let transfer = pack_roots(roots, &mut Symbols::default(), &[], WorkerNumber(0), &port);
    let roots = transfer
        .roots
        .iter()
        .map(|root| {
            let words = if matches!(
                root.words,
                ryo_values::domain::records::types::RootWords::Published(_)
            ) {
                "published"
            } else {
                "withheld"
            };
            let readiness = match &root.readiness {
                ProducerReadiness::Unknown(_) => "unknown",
                ProducerReadiness::Skipped => "skipped",
                ProducerReadiness::Complete => "complete",
                ProducerReadiness::NoQuery => "no query",
            };
            format!("{words} {readiness}")
        })
        .collect::<Vec<_>>();
    let observed = format!(
        "arena {:?}, {} call(s), {}",
        transfer.arena.0,
        port.0.load(Ordering::SeqCst),
        roots.join(", ")
    );
    Ok(Box::leak(observed.into_boxed_str()))
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
