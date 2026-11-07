//! A port is called for a file, never for a row: `lint` over the corpus the fixtures state, with
//! the real Rust language and real queries, calls the facts port as often when every file has ten
//! times the rows. The counting and the comparison are the core's contract, the same for every product.

use std::path::Path;
use std::sync::Arc;

use ryo_core::inbound::use_cases::lint::fixtures::contract;
use ryo_core::outbound::language::factory;
use ryo_core::outbound::language::factory::LanguageConnection;

#[test]
fn port_calls_for_a_file_do_not_grow_with_its_rows() {
    let rust = ryo_rust::linked();
    let corpus = contract::corpus(
        Arc::new(factory::use_case(vec![LanguageConnection::from(rust)])),
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../languages/rust/tests/fixtures"),
        &ryo_rust::fixtures::expected(),
    );
    contract::call_count(&corpus);
}
