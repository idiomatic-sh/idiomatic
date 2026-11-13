//! The `receipts` port's fake runs the port's contract.
use super::super::contract::check;
use super::ReceiptsFake;

#[test]
fn succeeds() {
    let fake = ReceiptsFake::default();
    check(&fake, &fake.case_receipts).unwrap();
}
