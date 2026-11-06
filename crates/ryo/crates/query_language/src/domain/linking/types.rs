//! Plain linker indexes over parsed query bundle values.

use std::borrow::Borrow;
use std::collections::BTreeMap;

use ryo_values::domain::program::types::{ArgumentValue, ImportDeclaration};

/// The local alias an import is visible under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct ImportAlias(pub Box<str>);

impl Borrow<str> for ImportAlias {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Imported aliases visible from one source document.
#[derive(Clone, Debug, Default)]
pub(super) struct Imported(pub BTreeMap<ImportAlias, ImportDeclaration>);

impl Imported {
    /// Records an import under its alias and answers the one it replaced.
    pub(super) fn insert(
        &mut self,
        alias: Box<str>,
        import: ImportDeclaration,
    ) -> Option<ImportDeclaration> {
        self.0.insert(ImportAlias(alias), import)
    }

    /// The import written under `alias`.
    pub(super) fn get(&self, alias: &str) -> Option<&ImportDeclaration> {
        self.0.get(alias)
    }

    /// Every import, by alias order.
    pub(super) fn values(&self) -> impl Iterator<Item = &ImportDeclaration> {
        self.0.values()
    }
}

/// The name a fragment input or parameter is bound under.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct BindingName(pub Box<str>);

impl Borrow<str> for BindingName {
    fn borrow(&self) -> &str {
        &self.0
    }
}

/// Substitutions for a fragment's relation and scalar inputs.
#[derive(Clone, Debug, Default)]
pub(super) struct Bindings(pub BTreeMap<BindingName, ArgumentValue>);

impl Bindings {
    /// No substitutions.
    pub(super) fn new() -> Self {
        Self::default()
    }

    /// Binds `name` and answers the value it replaced.
    pub(super) fn insert(&mut self, name: Box<str>, value: ArgumentValue) -> Option<ArgumentValue> {
        self.0.insert(BindingName(name), value)
    }

    /// The value bound to `name`.
    pub(super) fn get(&self, name: &str) -> Option<&ArgumentValue> {
        self.0.get(name)
    }

    /// Whether `name` is bound.
    pub(super) fn contains_key(&self, name: &str) -> bool {
        self.0.contains_key(name)
    }
}
