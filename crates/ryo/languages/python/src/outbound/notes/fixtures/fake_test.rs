//! The `notes` port's fake runs the port's contract.

use super::super::contract::check;
use super::NotesFake;

#[test]
fn succeeds() {
    check(&NotesFake);
}
