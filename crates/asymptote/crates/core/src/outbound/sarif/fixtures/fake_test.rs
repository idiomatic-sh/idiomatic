//! The `sarif` port's fake answers what it was given, and refuses what it was not.

use super::{SarifFake, reading, result};
use crate::outbound::sarif::interface::SarifPort as _;
use crate::outbound::sarif::types::SarifText;

#[test]
fn succeeds() {
    let fake = reading("log", vec![result("a.rs", 1)]);
    assert_eq!(
        fake.read(&SarifText("log".into()))
            .map(|read| read.results.len()),
        Ok(1)
    );
}

#[test]
fn refuses() {
    assert!(
        SarifFake::default()
            .read(&SarifText(Box::default()))
            .is_err(),
        "an empty log is refused"
    );
}
