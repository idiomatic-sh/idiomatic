//! The `bmf` port's fake answers what it was given, and refuses what it was not.

use super::{BmfFake, measurement, reading};
use crate::outbound::bmf::errors::BmfError;
use crate::outbound::bmf::interface::BmfPort as _;
use crate::outbound::bmf::types::BmfText;

#[test]
fn succeeds() {
    let fake = reading("out", vec![measurement("b", "wall-ms", 1.0)]);
    assert_eq!(
        fake.read(&BmfText("out".into())).map(|read| read.0.len()),
        Ok(1)
    );
}

#[test]
fn refuses() {
    assert!(
        matches!(
            BmfFake::default().read(&BmfText(Box::default())),
            Err(BmfError::Unparsable { .. })
        ),
        "a text the fake was not given is unparsable"
    );
}
