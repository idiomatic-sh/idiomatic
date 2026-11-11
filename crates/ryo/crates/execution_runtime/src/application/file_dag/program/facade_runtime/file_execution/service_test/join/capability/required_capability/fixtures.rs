//! The support of the `required_capability` cases.
use super::super::super::super::super::*;
use super::super::super::*;
use super::super::fixtures::{Witness, refusal};
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::plans::types::Operation;
use std::sync::Arc;
use std::sync::atomic::Ordering;

pub(super) fn checked_refusal(op: &Operation, rows: [Rows<'_>; 2], symbols: &mut Symbols) {
    for delivery in [false, true] {
        for optional_limit in [None, Some(0)] {
            let witness = Arc::new(Witness {
                refuse: true,
                ..Witness::default()
            });
            let observed = invoke(
                op,
                rows.clone(),
                symbols,
                b"abcd",
                delivery,
                witness.clone(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .expect("real compiled owner");
            refusal(observed.result);
            assert!(observed.prefix.is_empty());
            assert_eq!(witness.calls.load(Ordering::Relaxed), 1);
            // A refusing dependency does not alter the immutable compiled meaning.
            let accepted = invoke(
                op,
                rows.clone(),
                symbols,
                b"abcd",
                delivery,
                real_port(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .unwrap();
            assert!(accepted.result.unwrap().count.0 > 0);
        }
    }
}
