//! Copy an input row followed by resolved appended values.
use ryo_relations::domain::relations::types::Value;

/// Fill the engine-owned scratch row without changing its allocation owner.
pub fn append(row: &[Value], appended: &[Value], target: &mut Vec<Value>) {
    target.clear();
    target.extend_from_slice(row);
    target.extend_from_slice(appended);
}

/// Appends the resolved values of an extension to a scheduled row.
pub(crate) fn evaluate(
    row: &[Value],
    appended: Option<&[Value]>,
    driver: &mut dyn crate::domain::evaluation::types::InvocationDriver,
    joined: &mut Vec<Value>,
) -> Result<(), crate::domain::contract::types::PortableOperatorFault> {
    let appended =
        appended.ok_or(crate::domain::contract::types::PortableOperatorFault::WrongBinding)?;
    append(row, appended, joined);
    driver.emit(joined)
}
