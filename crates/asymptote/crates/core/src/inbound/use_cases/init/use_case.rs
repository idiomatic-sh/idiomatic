//! The `init` use case: the product's configuration where it has none, and one script for each hook it lists. Every
//! hook is inspected before anything is written, so a script that is not asymptote's refuses the whole run.

use super::errors::InitError;
use super::interface::InitUseCase;
use super::types::{Configuration, InitPorts, InitReport, InitRequest};
use crate::domain::hooks::service::{ownership, script};
use crate::domain::hooks::types::Ownership;
use crate::domain::manifest::constants::{DEFAULT_GROUP, DEFAULT_HOOK, HOOKS};
use crate::domain::manifest::types::{Group, Hook, HookName, Manifest};
use crate::domain::operations::types::Operation;
use crate::domain::ratchet::types::Writing;
use crate::outbound::hooks::types::Installed;
use crate::outbound::manifest::errors::ManifestError;

/// `init`, over the ports the edge injects.
#[derive(Debug)]
pub struct InitUseCaseImpl {
    /// The outside `init` writes: the configuration, the lock's first operation and the git hooks.
    ports: InitPorts,
}

impl InitUseCaseImpl {
    /// `init` over the ports the edge injects.
    #[must_use]
    pub const fn new(ports: InitPorts) -> Self {
        Self { ports }
    }
}

impl InitUseCase for InitUseCaseImpl {
    fn init(&self, request: InitRequest) -> Result<InitReport, InitError> {
        let (configuration, listed): (Configuration, Vec<Hook>) =
            match self.ports.manifest.hooks(&request.start) {
                Ok(listed) => (Configuration::Kept, listed.0),
                Err(ManifestError::Missing { .. }) => (
                    Configuration::Created,
                    vec![Hook {
                        name: HookName(DEFAULT_HOOK.into()),
                        groups: vec![Group(DEFAULT_GROUP.into())],
                    }],
                ),
                Err(error) => return Err(error.into()),
            };
        for hook in &listed {
            if let Installed::Script { file, text } =
                self.ports.hooks.inspect(&request.start, &hook.name)?
                && ownership(&text) == Ownership::Foreign
            {
                return Err(InitError::Foreign {
                    hook: hook.name.0.to_string(),
                    file: file.0.to_string(),
                });
            }
        }
        let mut removed = Vec::new();
        for name in HOOKS {
            let name = HookName(name.into());
            if listed.iter().any(|hook| hook.name == name) {
                continue;
            }
            if let Installed::Script { text, .. } =
                self.ports.hooks.inspect(&request.start, &name)?
                && ownership(&text) == Ownership::Ours
            {
                removed.push(name);
            }
        }
        let installed: Vec<HookName> = listed.into_iter().map(|hook| hook.name).collect();
        // A configuration is written where the hook runs from: the repository's top level, not the directory `init` ran in.
        let top = self.ports.hooks.top(&request.start)?;
        let root = if configuration == Configuration::Created {
            top
        } else {
            request.start.clone()
        };
        if request.writing == Writing::Write {
            if configuration == Configuration::Created {
                // The default hook, and nothing else: the adapter writes it as its format does.
                self.ports.manifest.create(
                    &root,
                    &Manifest {
                        hooks: vec![Hook {
                            name: HookName(DEFAULT_HOOK.into()),
                            groups: vec![Group(DEFAULT_GROUP.into())],
                        }],
                        ..Manifest::default()
                    },
                )?;
                self.ports.lock.append(&Operation {
                    parents: Vec::new(),
                    covers: Vec::new(),
                    accepted: Vec::new(),
                    removed: Vec::new(),
                    perf: Vec::new(),
                    perf_removed: Vec::new(),
                })?;
            }
            for name in &installed {
                self.ports
                    .hooks
                    .install(&request.start, name, &script(name))?;
            }
            for name in &removed {
                self.ports.hooks.remove(&request.start, name)?;
            }
        }
        Ok(InitReport {
            root,
            configuration,
            installed,
            removed,
            writing: request.writing,
        })
    }
}
