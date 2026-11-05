//! Compose the module context filesystem boundary.
/// Package marker lookup with explicit repository isolation.
pub(crate) fn filesystem() -> impl super::interface::ModuleContextPort {
    super::adapters::filesystem::adapter::ModuleContextFilesystem
}
