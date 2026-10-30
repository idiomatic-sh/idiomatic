//! The `tools` port's fake runs the port's contract.

use super::super::contract::check;
use super::ToolsFake;
use crate::domain::manifest::types::ProductRoot;

#[test]
fn succeeds() {
    let fake = ToolsFake::default()
        .answering("echo", 0, "out\n")
        .answering("false", 1, "");
    check(&fake, &ProductRoot("/fake".into()));
}
