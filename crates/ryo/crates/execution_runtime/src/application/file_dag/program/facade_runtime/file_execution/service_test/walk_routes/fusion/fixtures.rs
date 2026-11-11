//! The support of the `fusion` cases.
use super::super::super::super::*;
use super::super::fixtures::plans::Authored;
use super::super::fixtures::routes::{Combine, Feed, Fusion};
use super::super::fixtures::semantics::{Derived, MeaningSnapshot, Meanings};
use super::super::fixtures::{Population, Program, Residency, demand};
use ryo_execution_domain::domain::executable_program::fusion::types::{Barrier, Fusibility};
use ryo_language::domain::symbols::types::Symbols;
use std::sync::Arc;

/// What one run of a combining program decided: rows or error, and the faults it reached.
#[derive(Debug, PartialEq)]
pub(super) struct Decision {
    pub(super) answer: String,
    pub(super) reached: MeaningSnapshot,
}

pub(super) fn decide(
    combine: Combine,
    left: Feed,
    right: Feed,
    fusion: Fusion,
) -> Result<Decision, OperatingError> {
    let meanings = Arc::new(Meanings::default());
    let program = Program::new(
        Authored::combined(combine, left, right, fusion),
        Population::Three,
        Residency::Stable,
        meanings.clone(),
        Arc::new(Derived::new()),
    )?;
    // The projection after each head is the only thing the second reader keeps apart.
    let compiled = program.prepared.program();
    for head in [0usize, 3] {
        let projection = compiled.rules()[0].original_steps[head + 1]
            .as_ref()
            .map_err(Clone::clone)?
            .logical;
        let class = compiled.fusibility(projection);
        match fusion {
            Fusion::Fused => assert_eq!(class, Some(Fusibility::Fusable), "{combine:?}"),
            Fusion::Held => assert_eq!(
                class,
                Some(Fusibility::Barrier(Barrier::SharedConsumer)),
                "{combine:?}"
            ),
        }
    }
    let mut symbols = Symbols::default();
    let mut file = program.open(&mut symbols)?;
    let read = |relation| program.read(relation);
    let answer = match file.rule(RuleIndex(0), demand(), &read, &mut symbols)?.emit {
        RootOutcome::Complete(table) => format!("complete {:?} {}", table.cells, table.count.0),
        RootOutcome::Failed(error) => format!("failed {error:?}"),
        _ => "other".to_owned(),
    };
    file.finish()?;
    let mut reached = meanings.snapshot();
    // Projection calls are the continuation itself; every other call is observable order.
    reached.projects = super::super::fixtures::semantics::Calls(0);
    Ok(Decision { answer, reached })
}
