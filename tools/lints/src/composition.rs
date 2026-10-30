//! The composition root: registers the boundary lints and their late pass with rustc.

use crate::boundary::adapters::rustc::adapter::{
    BoundaryRustcAdapter, NOMINAL_BOUNDARY_TYPE, UNPROVEN_BOUNDARY_TYPE,
};

pub(crate) fn register(_session: &rustc_session::Session, store: &mut rustc_lint::LintStore) {
    store.register_lints(&[
        NOMINAL_BOUNDARY_TYPE,
        UNPROVEN_BOUNDARY_TYPE,
        crate::construction::COMPOSITION_CONSTRUCTION,
        crate::command_contract::COMMAND_SERVICE_CONTRACT,
    ]);
    store.register_late_pass(move |context| {
        Box::new(crate::construction::Construction::new(context))
    });
    store.register_late_pass(move |context| Box::new(BoundaryRustcAdapter::new(context)));
    store.register_late_pass(move |context| {
        Box::new(crate::command_contract::CommandContract::new(context))
    });
}
