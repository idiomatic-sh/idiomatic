//! Copy an input row followed by resolved appended values.
use ryo_relations::domain::relations::types::Value;

/// Fill the engine-owned scratch row without changing its allocation owner.
pub fn append(row: &[Value], appended: &[Value], target: &mut Vec<Value>) {
    target.clear();
    target.extend_from_slice(row);
    target.extend_from_slice(appended);
}
