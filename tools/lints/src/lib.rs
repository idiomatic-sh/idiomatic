#![feature(rustc_private)]

extern crate rustc_data_structures;
extern crate rustc_hir;
extern crate rustc_lint;
extern crate rustc_middle;
extern crate rustc_session;
extern crate rustc_span;

mod boundary;
mod command_contract;
mod composition;
mod construction;

dylint_linting::dylint_library!();

#[unsafe(no_mangle)]
pub fn register_lints(session: &rustc_session::Session, store: &mut rustc_lint::LintStore) {
    composition::register(session, store);
}
