//! The fake of the `hooks` port: the scripts of one repository, held in memory.

use std::collections::BTreeMap;
use std::sync::Mutex;

use crate::domain::hooks::types::ScriptText;
use crate::domain::manifest::types::{HookName, ProductRoot};
use crate::outbound::hooks::errors::HooksError;
use crate::outbound::hooks::interface::HooksPort;
use crate::outbound::hooks::types::{HookFile, Installed};

/// Where the fake pretends the hooks are kept.
const FAKE_DIRECTORY: &str = "/fake/.git/hooks/";

/// Whether the fake's repository answers.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Reliability {
    /// It does.
    #[default]
    Answers,
    /// Every call fails with this.
    Fails(HooksError),
}

/// What the core's tests stand in for `hooks` with.
#[derive(Debug, Default)]
pub struct HooksFake {
    /// The scripts, by hook.
    pub scripts: Mutex<BTreeMap<HookName, ScriptText>>,
    /// Whether it answers.
    pub reliability: Reliability,
    /// The top level of the repository it pretends to be.
    pub top: Toplevel,
}

/// Where the fake's repository starts.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum Toplevel {
    /// In the directory asked from.
    #[default]
    Asked,
    /// In this directory, above the one asked from.
    At(ProductRoot),
}

impl HooksFake {
    /// The scripts held.
    #[must_use]
    pub fn held(&self) -> BTreeMap<HookName, ScriptText> {
        self.scripts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }

    /// The fake holding `text` as the hook `name`.
    #[must_use]
    pub fn holding(self, name: &str, text: &str) -> Self {
        // A script held under the name before is replaced, as a file written over another.
        self.scripts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend([(HookName(name.into()), ScriptText(text.into()))]);
        self
    }
}

impl HooksPort for HooksFake {
    fn top(&self, root: &ProductRoot) -> Result<ProductRoot, HooksError> {
        if let Reliability::Fails(failure) = &self.reliability {
            return Err(failure.clone());
        }
        Ok(match &self.top {
            Toplevel::Asked => root.clone(),
            Toplevel::At(top) => top.clone(),
        })
    }

    fn inspect(&self, _root: &ProductRoot, name: &HookName) -> Result<Installed, HooksError> {
        if let Reliability::Fails(failure) = &self.reliability {
            return Err(failure.clone());
        }
        Ok(self
            .held()
            .get(name)
            .map_or(Installed::Absent, |text| Installed::Script {
                file: HookFile([FAKE_DIRECTORY, &name.0].concat().into()),
                text: text.clone(),
            }))
    }

    fn install(
        &self,
        _root: &ProductRoot,
        name: &HookName,
        script: &ScriptText,
    ) -> Result<(), HooksError> {
        if let Reliability::Fails(failure) = &self.reliability {
            return Err(failure.clone());
        }
        // The script installed before is replaced, as the adapter writes over the file.
        self.scripts
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .extend([(name.clone(), script.clone())]);
        Ok(())
    }

    fn remove(&self, _root: &ProductRoot, name: &HookName) -> Result<(), HooksError> {
        if let Reliability::Fails(failure) = &self.reliability {
            return Err(failure.clone());
        }
        // Removing a hook that was never there is no failure, as on disk.
        drop(
            self.scripts
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner)
                .remove(name),
        );
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
