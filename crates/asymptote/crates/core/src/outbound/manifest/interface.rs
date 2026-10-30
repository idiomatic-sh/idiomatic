//! What the `manifest` port asks of the outside, in the core's own vocabulary.

use super::errors::ManifestError;
use super::types::{HookTable, Products};
use crate::domain::manifest::types::{Manifest, ProductRoot};

/// What every `manifest` adapter implements.
pub trait ManifestPort: std::fmt::Debug + Send + Sync {
    /// The product whose `asymptote.toml` is in `start` or the nearest directory above it.
    ///
    /// # Errors
    ///
    /// `Missing` when no directory at or above `start` holds one.
    fn find(&self, start: &ProductRoot) -> Result<ProductRoot, ManifestError>;

    /// The tools the product's `asymptote.toml` names.
    ///
    /// # Errors
    ///
    /// `Unreadable` when it cannot be read, `Shape` when it is not the shape asymptote reads.
    fn read(&self, root: &ProductRoot) -> Result<Manifest, ManifestError>;

    /// The member products the workspace rooted at `root` names, sorted by path: each directory its `members` globs
    /// match, each holding an `asymptote.toml` of its own that declares no `members`.
    ///
    /// # Errors
    ///
    /// `Member` when a glob matches no directory, a path does not exist, a directory has no `asymptote.toml`, or a
    /// member declares `members`; `NotRoot` when an `asymptote.toml` above `root` is the workspace root;
    /// `Unreadable` or `Shape` when a member's file cannot be read.
    fn members(&self, root: &ProductRoot) -> Result<Products, ManifestError>;

    /// The workspace root `start` belongs to: the topmost directory at or above its product that holds an
    /// `asymptote.toml`.
    ///
    /// # Errors
    ///
    /// `Missing` when no directory at or above `start` holds one.
    fn topmost(&self, start: &ProductRoot) -> Result<ProductRoot, ManifestError>;

    /// The hooks the workspace root of `start` maps, the default when its file has no `[hooks]` table, each of a
    /// group some tool of the workspace declares.
    ///
    /// # Errors
    ///
    /// `Missing`, `Unreadable` or `Shape` when the root's file cannot be found or read; `Hook` when a hook maps a
    /// group no tool of the workspace declares, while it declares tools.
    fn hooks(&self, start: &ProductRoot) -> Result<HookTable, ManifestError>;

    /// Writes `manifest` as the `asymptote.toml` of the product at `root`, which holds none.
    ///
    /// # Errors
    ///
    /// `Unwritable` when it cannot be written or one is there already.
    fn create(&self, root: &ProductRoot, manifest: &Manifest) -> Result<(), ManifestError>;
}
