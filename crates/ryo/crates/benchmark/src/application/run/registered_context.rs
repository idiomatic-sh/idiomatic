//! Registered verification uses every helper declared by the pinned context profile.
use crate::outbound::inventory::{
    interface::InventoryPort,
    types::{Command, RegisteredRule},
};

/// Validate the full context and its exhaustive module declarations, before and after native execution.
pub(crate) fn verify(entry: &RegisteredRule, store: &dyn InventoryPort) -> Result<(), String> {
    let Command::CaseVerification { context } = entry
        .command
        .as_ref()
        .ok_or(crate::application::run::constants::MISSING_VERIFICATION_COMMAND)?;
    let actual = store.inspect_context(context, &entry.artifacts)?;
    if !actual.modules.keys().eq(entry.modules.keys()) {
        return Err(crate::application::run::constants::REGISTERED_MODULE_INVENTORY_DIFFERS_FROM_PIN.into());
    }
    for (name, relative) in actual.modules {
        let pin = entry
            .modules
            .get(&name)
            .ok_or(crate::application::run::constants::MISSING_DECLARED_MODULE_PIN)?;
        if pin.path != context.join(relative) || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
        {
            return Err(format!(
                "registered module pin differs from pinned context profile: {name}"
            ));
        }
    }
    Ok(())
}
