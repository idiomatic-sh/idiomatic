//! Preserved validation diagnostics do not make literal rows incomplete.

use super::super::fixtures::contract::check;
use super::item::LiteralPropertiesItem;
use crate::fixtures::reading::reading;

#[test]
fn module_nonlocal_validation_keeps_literal_properties_complete() {
    check::<LiteralPropertiesItem>();
    let result = reading::<LiteralPropertiesItem>("nonlocal x\n1\n");
    assert!(result.causes.is_empty());
    assert_eq!(result.rows.len(), 1);
}
