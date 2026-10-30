//! The fake of the `manifest` port: one product, held in memory.

use crate::domain::manifest::types::{
    Argument, Argv, Fix, Group, Hook, HookName, Manifest, Output, Pathspec, ProductRoot, Seconds,
    Tool,
};
use crate::domain::violations::types::ToolId;
use crate::outbound::manifest::errors::ManifestError;
use crate::outbound::manifest::interface::ManifestPort;
use crate::outbound::manifest::types::{HookTable, Products};

/// What the core's tests stand in for `manifest` with.
#[derive(Debug, Default)]
pub struct ManifestFake {
    /// The product's root.
    pub root: ProductRoot,
    /// Its tools.
    pub manifest: Manifest,
    /// What was written, as `root`'s manifest.
    pub written: std::sync::Mutex<Vec<(ProductRoot, Manifest)>>,
}

impl ManifestFake {
    /// A product at `root` whose tools are `ids`, each read as SARIF and in the `lint` group.
    #[must_use]
    pub fn of(root: &str, ids: &[&str]) -> Self {
        Self {
            root: ProductRoot(root.into()),
            manifest: Manifest {
                asymptote: crate::domain::manifest::types::Pin::Unpinned,
                tools: ids
                    .iter()
                    .map(|id| Tool {
                        id: ToolId((*id).into()),
                        run: Argv(vec![Argument((*id).into())]),
                        fix: Fix::Absent,
                        output: Output::Sarif,
                        group: Group("lint".into()),
                        timeout: Seconds(60),
                        inputs: vec![Pathspec(".".into())],
                    })
                    .collect(),
                members: Vec::new(),
                hooks: vec![Hook {
                    name: HookName("pre-commit".into()),
                    groups: vec![Group("lint".into())],
                }],
            },
            written: std::sync::Mutex::default(),
        }
    }
}

impl ManifestPort for ManifestFake {
    fn find(&self, start: &ProductRoot) -> Result<ProductRoot, ManifestError> {
        if start.0.starts_with(&*self.root.0) {
            Ok(self.root.clone())
        } else {
            Err(ManifestError::Missing {
                start: start.0.to_string(),
            })
        }
    }

    fn read(&self, _root: &ProductRoot) -> Result<Manifest, ManifestError> {
        Ok(self.manifest.clone())
    }

    fn members(&self, _root: &ProductRoot) -> Result<Products, ManifestError> {
        Ok(Products::default())
    }

    fn topmost(&self, start: &ProductRoot) -> Result<ProductRoot, ManifestError> {
        self.find(start)
    }

    fn hooks(&self, start: &ProductRoot) -> Result<HookTable, ManifestError> {
        self.find(start)
            .map(|_| HookTable(self.manifest.hooks.clone()))
    }

    fn create(&self, root: &ProductRoot, manifest: &Manifest) -> Result<(), ManifestError> {
        self.written
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((root.clone(), manifest.clone()));
        Ok(())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
