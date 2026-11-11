//! The local pattern caches a real compiled owner measured, read from the runtime's own route observation: only patterns bound
//! at run time enter them.
use super::fixtures::{CASES, subject};
use ryo_values::cases::{Word, run};

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
