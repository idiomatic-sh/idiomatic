//! The demand program keeps the admissions in the order the compiler gave them.

use super::value::DemandProgram;
use crate::domain::interpreted_program::types::{DemandAction, InputId};

#[test]
fn the_admissions_are_kept_in_their_compiled_order() {
    let program = DemandProgram::new(
        vec![
            DemandAction::Admit(InputId(1)),
            DemandAction::Admit(InputId(0)),
        ]
        .into_boxed_slice(),
    );
    assert_eq!(
        program.actions(),
        [
            DemandAction::Admit(InputId(1)),
            DemandAction::Admit(InputId(0))
        ]
    );
    assert!(DemandProgram::new(Box::new([])).actions().is_empty());
}
