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
        .ok_or("missing verification command")?;
    let actual = store.inspect_context(context, &entry.artifacts)?;
    if !actual.modules.keys().eq(entry.modules.keys()) {
        return Err("registered module inventory differs from pinned context profile".into());
    }
    for (name, relative) in actual.modules {
        let pin = entry
            .modules
            .get(&name)
            .ok_or("missing declared module pin")?;
        if pin.path != context.join(relative) || entry.artifacts.get(&pin.path) != Some(&pin.sha256)
        {
            return Err(format!(
                "registered module pin differs from pinned context profile: {name}"
            ));
        }
    }
    Ok(())
}
